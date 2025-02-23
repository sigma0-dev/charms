use crate::{NormalizedSpell, Proof, CURRENT_VERSION, V0, V0_SPELL_VK};
use anyhow::{anyhow, bail, ensure};
use bitcoin::{
    hashes::{serde::Serialize, Hash},
    opcodes::all::{OP_ENDIF, OP_IF},
    script::{Instruction, PushBytes},
    TxIn,
};
use charms_data::{util, TxId, UtxoId};
use sp1_primitives::io::SP1PublicValues;
use sp1_verifier::Groth16Verifier;

/// Extract a [`NormalizedSpell`] from a transaction and verify it.
/// Incorrect spells are rejected.
pub fn extract_and_verify_spell(
    tx: &bitcoin::Transaction,
    spell_vk: &str,
) -> anyhow::Result<NormalizedSpell> {
    let Some((spell_tx_in, tx_ins)) = tx.input.split_last() else {
        bail!("transaction does not have inputs")
    };

    let script_data = spell_tx_in
        .witness
        .nth(1)
        .ok_or(anyhow!("no spell data in the last input's witness"))?;

    let (spell, proof) = parse_spell_and_proof(script_data)?;

    ensure!(
        &spell.tx.outs.len() <= &tx.output.len(),
        "spell tx outs mismatch"
    );
    ensure!(
        &spell.tx.ins.is_none(),
        "spell must inherit inputs from the enchanted tx"
    );

    let spell = spell_with_ins(spell, tx_ins);

    let (spell_vk, groth16_vk) = vks(spell.version, spell_vk)?;

    Groth16Verifier::verify(
        &proof,
        to_sp1_pv(spell.version, &(spell_vk, &spell)).as_slice(),
        spell_vk,
        groth16_vk,
    )
    .map_err(|e| anyhow!("could not verify spell proof: {}", e))?;

    Ok(spell)
}

fn spell_with_ins(spell: NormalizedSpell, spell_tx_ins: &[TxIn]) -> NormalizedSpell {
    let tx_ins = spell_tx_ins // exclude spell commitment input
        .iter()
        .map(|tx_in| {
            let out_point = tx_in.previous_output;
            UtxoId(TxId(out_point.txid.to_byte_array()), out_point.vout)
        })
        .collect();

    let mut spell = spell;
    spell.tx.ins = Some(tx_ins);

    spell
}

pub fn parse_spell_and_proof(script_data: &[u8]) -> anyhow::Result<(NormalizedSpell, Proof)> {
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

    let (spell, proof): (NormalizedSpell, Proof) = util::read(spell_data.as_slice())
        .map_err(|e| anyhow!("could not parse spell and proof: {}", e))?;
    Ok((spell, proof))
}

fn vks(spell_version: u32, spell_vk: &str) -> anyhow::Result<(&str, &[u8])> {
    match spell_version {
        CURRENT_VERSION => Ok((spell_vk, *sp1_verifier::GROTH16_VK_BYTES)),
        V0 => Ok((V0_SPELL_VK, V0_GROTH16_VK_BYTES)),
        _ => bail!("unsupported spell version: {}", spell_version),
    }
}

const V0_GROTH16_VK_BYTES: &'static [u8] = include_bytes!("../vk/v0/groth16_vk.bin");

fn to_sp1_pv<T: Serialize>(spell_version: u32, t: &T) -> SP1PublicValues {
    let mut pv = SP1PublicValues::new();
    match spell_version {
        CURRENT_VERSION => {
            // we commit to CBOR-encoded tuple `(spell_vk, n_spell)`
            pv.write_slice(util::write(t).unwrap().as_slice());
        }
        V0 => {
            // we used to commit to the tuple `(spell_vk, n_spell)`, which was serialized internally
            // by SP1
            pv.write(t);
        }
        _ => unreachable!(),
    }
    pv
}
