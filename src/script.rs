use bitcoin::{
    constants::MAX_SCRIPT_ELEMENT_SIZE,
    opcodes::{
        all::{OP_CHECKSIG, OP_ENDIF, OP_IF},
        OP_FALSE,
    },
    script::{Builder, PushBytes},
    secp256k1::Secp256k1,
    taproot::{ControlBlock, LeafVersion, TaprootBuilder, TaprootSpendInfo},
    ScriptBuf, XOnlyPublicKey,
};
use serde::Serialize;

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
