use crate::spells::Spell;
use bitcoin::{
    bip32::{DerivationPath, Xpriv},
    constants::MAX_SCRIPT_ELEMENT_SIZE,
    hashes::{sha256, Hash},
    key::{Keypair, Parity, TweakedPublicKey, UntweakedKeypair},
    opcodes::{
        all::{
            OP_2DROP, OP_CHECKSIG, OP_CHECKSIGVERIFY, OP_DROP, OP_ENDIF, OP_EQUAL, OP_EQUALVERIFY,
            OP_IF, OP_SHA256,
        },
        OP_FALSE,
    },
    script::{Builder, PushBytes},
    secp256k1::Secp256k1,
    taproot::{ControlBlock, LeafVersion, TaprootBuilder, TaprootSpendInfo},
    Address, Network, PrivateKey, PublicKey, ScriptBuf, WitnessProgram, XOnlyPublicKey,
};
use charms_data::Data;
use serde::Serialize;

pub fn magic_address(public_key: XOnlyPublicKey, network: Network, script: ScriptBuf) -> Address {
    Address::from_witness_program(spell_witness_program(public_key, script), network)
}

pub fn spell_witness_program(public_key: XOnlyPublicKey, script: ScriptBuf) -> WitnessProgram {
    WitnessProgram::p2tr_tweaked(taproot_spend_info(public_key, script).output_key())
}

pub fn derive_private_key(
    xpriv: &Xpriv,
    derivation_path: &DerivationPath,
    network: Network,
) -> PrivateKey {
    let secp = Secp256k1::new();
    let child_key = xpriv.derive_priv(&secp, derivation_path).unwrap();
    let private_key = child_key.private_key;

    PrivateKey::new(private_key, network)
}

pub fn control_block(public_key: XOnlyPublicKey, script: ScriptBuf) -> ControlBlock {
    taproot_spend_info(public_key, script.clone())
        .control_block(&(script, LeafVersion::TapScript))
        .unwrap()
}

pub fn data_script(public_key: XOnlyPublicKey, data: &Vec<u8>) -> ScriptBuf {
    let builder = ScriptBuf::builder();
    push_envelope(builder, data)
        .push_slice(public_key.serialize())
        .push_opcode(OP_CHECKSIG)
        .into_script()
}

fn push_envelope(builder: Builder, data: &Vec<u8>) -> Builder {
    let mut builder = builder
        .push_opcode(OP_FALSE)
        .push_opcode(OP_IF)
        .push_slice(b"spell");
    for chunk in data.chunks(MAX_SCRIPT_ELEMENT_SIZE) {
        builder = builder.push_slice::<&PushBytes>(chunk.try_into().unwrap());
    }
    builder.push_opcode(OP_ENDIF)
}

pub fn taproot_spend_info(public_key: XOnlyPublicKey, script: ScriptBuf) -> TaprootSpendInfo {
    let secp256k1 = Secp256k1::new();
    TaprootBuilder::new()
        .add_leaf(0, script)
        .unwrap()
        .finalize(&secp256k1, public_key)
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;
    use std::str::FromStr;

    pub const NETWORK: Network = Network::Testnet4;

    // original address: tb1p38tsza94t9pak7g52zku06c7r9jqcjx67l4r7d7ehw0ch8fm8cysknej4p

    #[test]
    fn test_get_private_key() {
        let tprv = "tprv8ZgxMBicQKsPdY8xCZptFjJ8HV5TFQr7K3k5Ue1cKfgjR3GuXps3JbFFXP2rLKxQ84u93ZvaXemwpmQcYcLwBS9gSrkRuNMZMdjvwUAdgwU";
        let path = "m/86h/1h/0h/0/17";

        let xkey = Xpriv::from_str(tprv).unwrap();
        let derivation_path = DerivationPath::from_str(path).unwrap();

        let new_priv_key = derive_private_key(&xkey, &derivation_path, NETWORK);
        dbg!(new_priv_key.to_wif());

        let secp256k1 = Secp256k1::new();
        let keypair = Keypair::from_secret_key(&secp256k1, &new_priv_key.inner);
    }
}
