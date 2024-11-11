use crate::script::{control_block, data_script, taproot_spend_info};
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

/// `add_spell` adds `spell` to `tx`:
/// 1. it builds `commit_spell_tx` transaction which creates a *committed spell* Tapscript output
/// 2. then appends an input spending the *committed spell* to `tx`, and adds a witness for it.
///
/// `fee_rate` is used to compute the amount of sats necessary to fund this, which is exactly the
/// amount of the created Tapscript output.
///
/// `tx` (prior to modification) is assumed to already include the fee at `fee_rate`:
/// we're simply adding an input (and a change output) while maintaining the same fee rate for the
/// modified `tx`.
///
/// Return `[commit_spell_tx, tx]`.
///
/// Both `commit_spell_tx` and `tx` need to be signed.
pub fn add_spell(
    tx: Transaction,
    spell_data: &[u8],
    funding_out_point: OutPoint,
    funding_output_value: Amount,
    change_script_pubkey: ScriptBuf,
    fee_rate: FeeRate,
) -> [Transaction; 2] {
    let secp256k1 = Secp256k1::new();
    let keypair = Keypair::new(&secp256k1, &mut thread_rng());
    let (public_key, _) = XOnlyPublicKey::from_keypair(&keypair);

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
    let added_weight =
        Weight::from_witness_data_size(script_len as u64) + Weight::from_wu(263 + 168);

    // CPFP paying for commit_tx (111 vB) minus (already paid) relay fee of 111 sats
    let commit_tx_fee_cpfp = fee_rate.fee_vb(111).unwrap() - Amount::from_sat(111);

    fee_rate.fee_wu(added_weight).unwrap() + commit_tx_fee_cpfp
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
        let tx_hex = "02000000012f7d19323990772b01a9efc34f29bb110b1df2cd4acb2c1fd64dbd56ac0027f70100000000fdffffff02102700000000000022512002a094075fa87e65564ef2e9be5f1d9bb2c2c68060694fa9f262b134f7b3852b110007000000000022512035110fe9264022566504564fcfb0ce154bf5b66e4476739d5ffe7736afab798400000000";
        let tx = deserialize_hex::<Transaction>(tx_hex).unwrap();

        let [commit_tx, tx] = add_spell(
            dbg!(tx),
            b"awesome-spell",
            OutPoint {
                txid: Txid::from_str(
                    "f72700ac56bd4dd61f2ccb4acdf21d0b11bb294fc3efa9012b77903932197d2f",
                )
                .unwrap(),
                vout: 0,
            },
            Amount::from_sat(10000),
            Address::from_str("tb1pn8dcuyac5z5cyck7audhk8gkj6zz4fh4l0jv5cws9w68szyaa3ksqgdanl")
                .unwrap()
                .assume_checked()
                .script_pubkey(),
            FeeRate::from_sat_per_vb(2u64).unwrap(),
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
