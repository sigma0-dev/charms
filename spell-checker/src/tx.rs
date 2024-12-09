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
    let script_data = tx.input[tx.input.len() - 1]
        .witness
        .nth(1)
        .ok_or(anyhow!("no spell data in the last witness"))?;

    // Parse script_data into Script
    let script = bitcoin::blockdata::script::Script::from_bytes(script_data);

    let mut instructions = script.instructions();

    ensure!(instructions.next() == Some(Ok(Instruction::PushBytes(PushBytes::empty()))));
    ensure!(instructions.next() == Some(Ok(Instruction::Op(OP_IF))));
    let Some(Ok(Instruction::PushBytes(push_bytes))) = instructions.next() else {
        bail!("no spell")
    };
    if push_bytes.as_bytes() != b"spell" {
        bail!("no spell")
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
                bail!("no spell")
            }
        }
    }

    let (spell, proof): (NormalizedSpell, Proof) =
        ciborium::de::from_reader(spell_data.as_slice())?;

    ensure!(
        &spell.tx.outs.len() <= &tx.output.len(),
        "spell tx outs mismatch"
    );
    ensure!(
        &spell.tx.ins.is_none(),
        "spell inherits inputs from the enchanted tx"
    );

    let tx_ins = tx.input[..tx.input.len() - 1]
        .iter()
        .map(|txin| {
            let out_point = txin.previous_output;
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
    )?;

    Ok((spell, proof))
}

fn to_sp1_pv<T: Serialize>(t: &T) -> SP1PublicValues {
    let mut pv = SP1PublicValues::new();
    pv.write(t);
    pv
}
