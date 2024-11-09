use crate::address::{control_block, data_script, taproot_spend_info};
use bitcoin::{
    self,
    absolute::LockTime,
    hex::FromHex,
    key::Secp256k1,
    secp256k1::{schnorr, Keypair, Message},
    sighash::{Prevouts, SighashCache},
    taproot,
    taproot::LeafVersion,
    transaction::Version,
    Address, Amount, FeeRate, OutPoint, ScriptBuf, TapLeafHash, TapSighashType, Transaction, TxIn,
    TxOut, Txid, Weight, Witness, XOnlyPublicKey,
};
use charms_data::Data;
use rand::thread_rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spell(pub Data);

/// Create `commit_spell_tx` which creates a Tapscript output. Append an input spending this output
/// to `tx`. `spell` is thus added to `tx`.
/// `fee_rate` is used to compute the amount of sats necessary to fund this, which is exactly the
/// amount of the created Tapscript output.
///
/// Return `[commit_spell_tx, tx]`.
/// `commit_spell_tx` needs to be funded: additional inputs must be **appended** to its list of
/// inputs.
pub fn add_spell(
    tx: Transaction,
    spell: &Spell,
    funding_out_point: OutPoint,
    funding_output_value: Amount,
    change_script_pubkey: ScriptBuf,
    fee_rate: FeeRate,
) -> [Transaction; 2] {
    let secp256k1 = Secp256k1::new();
    let keypair = Keypair::new(&secp256k1, &mut thread_rng());
    let (public_key, _) = XOnlyPublicKey::from_keypair(&keypair);

    let spell_data = postcard::to_stdvec(spell).unwrap();
    let script = data_script(public_key, &spell_data);
    let fee = compute_fee(fee_rate, script.len());

    let mut tx = tx;

    let (commit_spell_tx, committed_spell_txout) =
        create_commit_tx(funding_out_point, funding_output_value, public_key, &script);
    let commit_spell_txid = commit_spell_tx.compute_txid();
    let change_amount = committed_spell_txout.value - fee;

    modify_tx(
        &mut tx,
        commit_spell_txid,
        change_script_pubkey,
        change_amount,
    );
    let spell_input = tx.input.len() - 1;

    let signature = create_tx_signature(
        keypair,
        &mut tx,
        spell_input,
        &committed_spell_txout,
        &script,
    );

    append_witness_data(
        &mut tx.input[spell_input].witness,
        public_key,
        script,
        signature,
    );

    [commit_spell_tx, tx]
}

/// fee covering only the marginal cost of spending the committed spell output.
fn compute_fee(fee_rate: FeeRate, script_len: usize) -> Amount {
    // script input: (41 * 4) + (L + 99) = 164 + L + 99 = L + 263 wu
    // change output: 42 * 4 = 168 wu
    let weight = Weight::from_witness_data_size(script_len as u64) + Weight::from_wu(263 + 168);
    fee_rate.fee_wu(weight).unwrap()
}

fn create_commit_tx(
    funding_out_point: OutPoint,
    funding_output_value: Amount,
    public_key: XOnlyPublicKey,
    script: &ScriptBuf,
) -> (Transaction, TxOut) {
    const RELAY_FEE: Amount = Amount::from_sat(111); // assuming spending exactly 1 output via key path

    let committed_spell_txout = TxOut {
        value: funding_output_value - RELAY_FEE,
        script_pubkey: ScriptBuf::new_p2tr_tweaked(
            taproot_spend_info(public_key, script.clone()).output_key(),
        ),
    };
    let commit_spell_tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: funding_out_point,
            script_sig: Default::default(),
            sequence: Default::default(),
            witness: Default::default(),
        }],
        output: vec![committed_spell_txout.clone()],
    };
    (commit_spell_tx, committed_spell_txout)
}

fn modify_tx(
    tx: &mut Transaction,
    commit_spell_txid: Txid,
    change_script_pubkey: ScriptBuf,
    change_amount: Amount,
) {
    tx.input.push(TxIn {
        previous_output: OutPoint {
            txid: commit_spell_txid,
            vout: 0,
        },
        script_sig: Default::default(),
        sequence: Default::default(),
        witness: Witness::new(),
    });

    if change_amount >= Amount::from_sat(546) {
        // dust limit
        tx.output.push(TxOut {
            value: change_amount,
            script_pubkey: change_script_pubkey,
        });
    }
}

fn create_tx_signature(
    keypair: Keypair,
    tx: &mut Transaction,
    input_index: usize,
    prev_out: &TxOut,
    script: &ScriptBuf,
) -> schnorr::Signature {
    let mut sighash_cache = SighashCache::new(tx);
    let sighash = sighash_cache
        .taproot_script_spend_signature_hash(
            input_index,
            &Prevouts::One(input_index, prev_out),
            TapLeafHash::from_script(script, LeafVersion::TapScript),
            TapSighashType::AllPlusAnyoneCanPay,
        )
        .unwrap();
    let secp256k1 = Secp256k1::new();
    let signature = secp256k1.sign_schnorr(
        &Message::from_digest_slice(sighash.as_ref())
            .expect("should be cryptographically secure hash"),
        &keypair,
    );

    signature
}

fn append_witness_data(
    witness: &mut Witness,
    public_key: XOnlyPublicKey,
    script: ScriptBuf,
    signature: schnorr::Signature,
) {
    witness.push(
        taproot::Signature {
            signature,
            sighash_type: TapSighashType::AllPlusAnyoneCanPay,
        }
        .to_vec(),
    );
    witness.push(script.clone());
    witness.push(control_block(public_key, script).serialize());
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::{
        consensus::encode::{deserialize_hex, serialize_hex},
        Amount, Network, Txid,
    };
    use std::str::FromStr;

    pub const NETWORK: Network = Network::Testnet4;

    #[test]
    fn test_add_spell() {
        let tx_hex = "020000000113be81060725ffa5f2764aeecbc6312c030525e8c1f4541067748fea255423f00100000000fdffffff021027000000000000225120a94135dccd5b2734ca9af5c19f79feea419cf066c20c814daeed41a24c244dd457280700000000002251204f07be0503523927737f3c075a9035f2abb33f8f14c9036f6e0af3dde111d41c00000000";
        let tx = deserialize_hex::<Transaction>(tx_hex).unwrap();

        let [commit_tx, tx] = add_spell(
            dbg!(tx),
            &Spell(Data(b"awesome-spell".to_vec())),
            OutPoint {
                txid: Txid::from_str(
                    "f0235425ea8f74671054f4c1e82505032c31c6cbee4a76f2a5ff25070681be13",
                )
                .unwrap(),
                vout: 0,
            },
            Amount::from_sat(10000),
            Address::from_str("tb1py54d5l6y9y4uszgaj05wv4su2tnedvstny9y0va940d0h5upezjq84m89p")
                .unwrap()
                .assume_checked()
                .script_pubkey(),
            FeeRate::from_sat_per_vb(1u64).unwrap(),
        );

        let commit_tx_hex = serialize_hex(dbg!(&commit_tx));
        let serialized = bitcoin::consensus::encode::serialize(&commit_tx);

        dbg!(&commit_tx_hex);
        let decoded_commit_tx = deserialize_hex::<Transaction>(commit_tx_hex.as_ref()).unwrap();
        assert_eq!(commit_tx, decoded_commit_tx);

        let tx_hex = serialize_hex(&tx);
        dbg!(tx_hex);
    }
}
