use crate::address::{
    control_block, data_script, magic_address, spell_witness_program, taproot_spend_info,
};
use anyhow::Result;
use bitcoin::{
    self,
    absolute::LockTime,
    hex,
    hex::FromHex,
    key::Secp256k1,
    opcodes, script,
    script::PushBytesBuf,
    secp256k1::{schnorr, Keypair, Message},
    sighash::{Prevouts, SighashCache},
    taproot::{LeafVersion, Signature},
    transaction::Version,
    Address, Amount, FeeRate, OutPoint, PrivateKey, ScriptBuf, Sequence, TapLeafHash,
    TapSighashType, Transaction, TxIn, TxOut, Weight, Witness, XOnlyPublicKey,
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
    change_address: Address,
    fee_rate: FeeRate,
) -> [Transaction; 2] {
    let mut tx = tx;

    let secp256k1 = Secp256k1::new();

    let keypair = Keypair::new(&secp256k1, &mut thread_rng());
    let (public_key, _) = XOnlyPublicKey::from_keypair(&keypair);

    let spell_data = postcard::to_stdvec(spell).unwrap();
    let script = data_script(public_key, &spell_data);

    let script_len = script.len();

    const RELAY_FEE: Amount = Amount::from_sat(111);

    let fee = compute_fee(fee_rate, script_len);

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

    let spell_input_index = tx.input.len();
    tx.input.push(TxIn {
        previous_output: OutPoint {
            txid: commit_spell_tx.compute_txid(),
            vout: 0,
        },
        script_sig: Default::default(),
        sequence: Default::default(),
        witness: Witness::new(),
    });

    let change_amount = committed_spell_txout.value - fee;
    if change_amount >= Amount::from_sat(546) {
        // dust limit
        tx.output.push(TxOut {
            value: change_amount,
            script_pubkey: change_address.script_pubkey(),
        });
    }

    let signature: schnorr::Signature = create_signature(
        keypair,
        &script,
        &mut tx,
        spell_input_index,
        &committed_spell_txout,
    );

    let witness = &mut tx.input[spell_input_index].witness;
    witness.push(
        Signature {
            signature,
            sighash_type: TapSighashType::AllPlusAnyoneCanPay,
        }
        .to_vec(),
    );
    witness.push(script.clone());
    witness.push(control_block(public_key, script).serialize());

    [commit_spell_tx, tx]
}

/// fee covering only the marginal cost of spending the committed spell output.
fn compute_fee(fee_rate: FeeRate, script_len: usize) -> Amount {
    let weight = Weight::from_witness_data_size(script_len as u64) + Weight::from_wu(702);
    fee_rate.fee_wu(weight).unwrap()
}

fn create_signature(
    keypair: Keypair,
    script: &ScriptBuf,
    tx: &mut Transaction,
    input_index: usize,
    prev_out: &TxOut,
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

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::{
        consensus::encode::{deserialize_hex, serialize_hex},
        hex::DisplayHex,
        opcodes::all::OP_PUSHNUM_1,
        Amount, Network, Txid,
    };
    use std::str::FromStr;

    pub const NETWORK: Network = Network::Testnet4;

    #[test]
    fn test_add_spell() {
        let tx_hex = "020000000146d21179c0a2036e4eb1fa8f18b6f6f9dd785a1f87e67dfdede73a0c237227930000000000fdffffff01e8030000000000002251201ff2305008a9ebf588f5e2fdfdff0be266bd051fadb3d869b84a3bf7ce6374d800000000";
        let tx = deserialize_hex::<Transaction>(tx_hex).unwrap();

        let [commit_tx, tx] = add_spell(
            dbg!(tx),
            &Spell(Data(b"awesome-spell".to_vec())),
            OutPoint {
                txid: Txid::from_str(
                    "e36ef9e4f618463ddf6bcbc23e691a0f9e3ae590c770c06a7b92c5d29be7f9f3",
                )
                .unwrap(),
                vout: 1,
            },
            Amount::from_sat(494000),
            Address::from_str("tb1pgkwtn34z7kz4kuzaxpp6yx6n2qnykret97np0j8xqpk938qx6t4sza9lyf")
                .unwrap()
                .assume_checked(),
            FeeRate::from_sat_per_vb(1u64).unwrap(),
        );

        let commit_tx_hex = serialize_hex(dbg!(&commit_tx));
        let serialized = bitcoin::consensus::encode::serialize(&commit_tx);

        dbg!(&commit_tx_hex);
        let decoded_commit_tx = deserialize_hex::<Transaction>(commit_tx_hex.as_ref()).unwrap();
        assert_eq!(commit_tx, decoded_commit_tx);

        let tx_hex = serialize_hex(&tx);
        dbg!(tx_hex);

        // let [_, tx] = add_spell(
        //     &Spell(Data(b"awesome-spell".to_vec())),
        //     tx,
        //     &[prev_tx_out],
        //     0,
        // );
        //
        // let tx_with_spell = serialize_hex(&tx);
        // dbg!(tx_with_spell);
    }
}
