use crate::commands::SpellCommands;
use anyhow::{anyhow, ensure, Result};
use bitcoin::{consensus::encode::deserialize_hex, hashes::Hash, Transaction};
use charms::{app, spell, spell::Spell, SPELL_VK};
use charms_data::{TxId, VkHash};
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

    let tx = deserialize_hex::<Transaction>(&tx)?;

    let (norm_spell, app_private_inputs) = spell.normalized()?;

    let spell_ins = norm_spell.tx.ins.as_ref().ok_or(anyhow!("no inputs"))?;
    for i in 0..spell_ins.len() {
        let utxo_id = &spell_ins[i];
        let out_point = tx.input[i].previous_output;
        ensure!(
            utxo_id.0 == TxId(out_point.txid.to_byte_array()),
            "input {} txid mismatch: {} != {}",
            i,
            utxo_id.0,
            out_point.txid
        );
        ensure!(
            utxo_id.1 == out_point.vout,
            "input {} vout mismatch: {} != {}",
            i,
            utxo_id.1,
            out_point.vout
        );
    }

    let prev_txs = prev_txs
        .iter()
        .map(|prev_tx| {
            let prev_tx = deserialize_hex::<Transaction>(prev_tx)?;

            Ok((TxId(prev_tx.compute_txid().to_byte_array()), prev_tx))
        })
        .collect::<Result<BTreeMap<_, _>>>()?
        .into_values()
        .collect();

    let app_prover = app::Prover::new();

    let binaries = app_bins
        .iter()
        .map(|path| {
            let binary = std::fs::read(path)?;
            let vk_hash = app_prover.vk(&binary);
            Ok((VkHash(vk_hash), binary))
        })
        .collect::<Result<_>>()?;

    let (norm_spell, proof) = spell::prove(
        norm_spell,
        &binaries,
        app_private_inputs,
        prev_txs,
        &SPELL_VK,
    )?;

    ciborium::into_writer(&(&norm_spell, &proof), std::io::stdout())?;

    Ok(())
}
