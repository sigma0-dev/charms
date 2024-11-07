use crate::address::{control_block, spend_script};
use anyhow::Result;
use bitcoin::{
    consensus::{
        deserialize,
        encode::{deserialize_hex, serialize_hex},
    },
    hex,
    hex::FromHex,
    key::Secp256k1,
    opcodes, script,
    script::PushBytesBuf,
    secp256k1::{schnorr, Keypair, Message},
    sighash::{Prevouts, SighashCache},
    taproot::{LeafVersion, Signature},
    PrivateKey, ScriptBuf, TapLeafHash, TapSighashType, Transaction, TxOut, Witness,
    XOnlyPublicKey,
};
use charms_data::Data;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spell(Data);

pub fn add_spell(
    spell: &Spell,
    private_key: &PrivateKey,
    tx: Transaction,
    prevouts: &[TxOut],
    magic_input: usize,
) -> [Transaction; 2] {
    let mut tx = tx;

    let secp256k1 = Secp256k1::new();

    let keypair = Keypair::from_secret_key(&secp256k1, &private_key.inner);
    let (public_key, _) = XOnlyPublicKey::from_keypair(&keypair);
    let script = spend_script(public_key);
    let signature: schnorr::Signature =
        create_signature(keypair, &script, &mut tx, magic_input, prevouts);

    let witness = &mut tx.input[magic_input].witness;
    witness.push(
        Signature {
            signature,
            sighash_type: TapSighashType::Default,
        }
        .to_vec(),
    );
    witness.push(b"spell");
    witness.push(postcard::to_stdvec(spell).unwrap());
    witness.push(script);
    witness.push(control_block(public_key).serialize());

    let commit_tx = create_commit_transaction();
    [commit_tx, tx]
}

fn create_signature(
    keypair: Keypair,
    script: &ScriptBuf,
    tx: &mut Transaction,
    magic_input: usize,
    prevouts: &[TxOut],
) -> schnorr::Signature {
    let mut sighash_cache = SighashCache::new(tx);
    let sighash = sighash_cache
        .taproot_script_spend_signature_hash(
            magic_input,
            &Prevouts::All(&prevouts),
            TapLeafHash::from_script(script, LeafVersion::TapScript),
            TapSighashType::Default,
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
    use bitcoin::{opcodes::all::OP_PUSHNUM_1, Amount};

    #[test]
    fn test_add_spell() {
        let tx_hex = "0200000001f3f9e79bd2c5927b6ac070c790e53a9e0f1a693ec2cb6bdf3d4618f6e4f96ee30000000000fdffffff01e803000000000000225120dd8d4f69c533283dd0f9ebcd3abab531e687cd53dd0b75713f2423d31116c60200000000";
        let prev_tx_out_xonly_pubkey = <[u8; 32]>::from_hex(
            "b1d32959a471093c08914c0d97ae1358300668ab9ef4731b72b8316f14b601c4",
        )
        .unwrap();
        let private_key_wif = "cRFub1V7havSzKzzf6DssxdjCNX3RABtdq9HZscQ7xi2UnUEU8ko";

        let private_key = PrivateKey::from_wif(private_key_wif).unwrap();
        let tx = deserialize_hex::<Transaction>(tx_hex).unwrap();
        let prev_tx_out = TxOut {
            value: Amount::from_sat(2000),
            script_pubkey: ScriptBuf::builder()
                .push_opcode(OP_PUSHNUM_1)
                .push_slice(prev_tx_out_xonly_pubkey)
                .into_script(),
        };

        let tx = add_spell(
            &Spell(Data(b"awesome-spell".to_vec())),
            &private_key,
            tx,
            &[prev_tx_out],
            0,
        );

        let tx_with_spell = serialize_hex(&tx);
        dbg!(tx_with_spell);
    }
}
