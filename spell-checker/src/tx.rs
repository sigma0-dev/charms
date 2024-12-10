use crate::{NormalizedSpell, Proof};
use anyhow::{anyhow, bail, ensure, Error};
use bitcoin::{
    hashes::Hash,
    opcodes::all::{OP_ENDIF, OP_IF},
    script::{Instruction, PushBytes},
};
use charms_data::{TxId, UtxoId};
use serde::Serialize;
use sp1_primitives::io::SP1PublicValues;
use sp1_verifier::Groth16Verifier;

pub fn extract_spell(
    tx: &bitcoin::Transaction,
    spell_vk: &str,
) -> anyhow::Result<(NormalizedSpell, Proof), Error> {
    let spell_input_index = tx.input.len() - 1;

    let script_data = tx.input[spell_input_index]
        .witness
        .nth(1)
        .ok_or(anyhow!("no spell data in the last input's witness"))?;

    // Parse script_data into Script
    let script = bitcoin::blockdata::script::Script::from_bytes(script_data);

    let mut instructions = script.instructions();

    ensure!(instructions.next() == Some(Ok(Instruction::PushBytes(PushBytes::empty()))));
    ensure!(instructions.next() == Some(Ok(Instruction::Op(OP_IF))));
    let Some(Ok(Instruction::PushBytes(push_bytes))) = instructions.next() else {
        bail!("no spell data")
    };
    if push_bytes.as_bytes() != b"spell" {
        bail!("no spell marker")
    }

    let mut spell_data = vec![];

    loop {
        match instructions.next() {
            Some(Ok(Instruction::PushBytes(push_bytes))) => {
                spell_data.extend(push_bytes.as_bytes());
            }
            Some(Ok(Instruction::Op(OP_ENDIF))) => {
                break;
            }
            _ => {
                bail!("unexpected opcode")
            }
        }
    }

    let (spell, proof): (NormalizedSpell, Proof) = ciborium::de::from_reader(spell_data.as_slice())
        .map_err(|e| anyhow!("could not parse spell and proof: {}", e))?;

    ensure!(
        &spell.tx.outs.len() <= &tx.output.len(),
        "spell tx outs mismatch"
    );
    ensure!(
        &spell.tx.ins.is_none(),
        "spell must inherit inputs from the enchanted tx"
    );

    let tx_ins = tx.input[..spell_input_index] // exclude spell commitment input
        .iter()
        .map(|tx_in| {
            let out_point = tx_in.previous_output;
            UtxoId(TxId(out_point.txid.to_byte_array()), out_point.vout)
        })
        .collect();

    let mut spell = spell;
    spell.tx.ins = Some(tx_ins);

    Groth16Verifier::verify(
        &proof,
        to_sp1_pv(&(spell_vk, &spell)).as_slice(),
        spell_vk,
        *sp1_verifier::GROTH16_VK_BYTES,
    )
    .map_err(|e| anyhow!("could not verify spell proof: {}", e))?;

    Ok((spell, proof))
}

fn to_sp1_pv<T: Serialize>(t: &T) -> SP1PublicValues {
    let mut pv = SP1PublicValues::new();
    pv.write(t);
    pv
}
