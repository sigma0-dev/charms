use crate::commands::SpellCommands;
use anyhow::Result;
use bitcoin::{consensus::encode::deserialize_hex, hashes::Hash, Transaction};
use charms::{app, spell, spell::Spell, tx};
use charms_data::{TxId, VkHash};
use spell_prover::{NormalizedSpell, NormalizedTransaction, V0};
use std::collections::BTreeMap;

pub fn spell_parse() -> Result<()> {
    let spell: Spell = serde_yaml::from_reader(std::io::stdin())?;
    ciborium::into_writer(&spell, std::io::stdout())?;

    Ok(())
}

pub fn spell_print() -> Result<()> {
    let spell: Spell = ciborium::de::from_reader(std::io::stdin())?;
    serde_yaml::to_writer(std::io::stdout(), &spell)?;

    Ok(())
}

pub fn spell_prove(command: SpellCommands) -> Result<()> {
    let SpellCommands::Prove {
        spell,
        tx,
        prev_txs,
        app_bins,
    } = command
    else {
        unreachable!()
    };

    dbg!(&tx);
    dbg!(&prev_txs);
    dbg!(&app_bins);

    sp1_sdk::utils::setup_logger();

    let spell: Spell = serde_yaml::from_slice(&std::fs::read(spell)?)?;
    dbg!(&spell);

    // TODO use tx in verifying the spell: it must be the same as the spell's tx
    // maybe put the hash of (tx's inputs (w/o the one with the spell) and number of outputs)
    // in the committed inputs of the proof
    let tx = deserialize_hex::<Transaction>(&tx)?;

    let (norm_spell, app_private_inputs) = spell.normalized()?;

    let prev_spells = prev_txs
        .iter()
        .map(|prev_tx| {
            let prev_tx = deserialize_hex::<Transaction>(prev_tx)?;

            let spell_and_proof_opt = tx::extract_spell(&prev_tx).ok();
            let (prev_spell, proof) = match spell_and_proof_opt {
                Some((spell, proof)) => (spell, Some(proof)),
                None => {
                    let spell = NormalizedSpell {
                        version: V0,
                        tx: NormalizedTransaction {
                            ins: None,
                            refs: Default::default(),
                            outs: vec![Default::default(); prev_tx.output.len()],
                        },
                        app_public_inputs: Default::default(),
                    };
                    (spell, None)
                }
            };

            let txid = prev_tx.compute_txid();
            let txid_bytes: [u8; 32] = txid.to_byte_array();

            Ok((TxId(txid_bytes), (prev_spell, proof)))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;

    let app_prover = app::Prover::new();

    let binaries = app_bins
        .iter()
        .map(|path| {
            let binary = std::fs::read(path)?;
            let vk_hash = app_prover.vk(&binary);
            Ok((VkHash(vk_hash), binary))
        })
        .collect::<Result<_>>()?;

    let (norm_spell, proof) = spell::prove(norm_spell, prev_spells, &binaries, app_private_inputs)?;

    ciborium::into_writer(&(&norm_spell, &proof), std::io::stdout())?;

    Ok(())
}
