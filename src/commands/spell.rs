use crate::commands::SpellCommands;
use anyhow::{anyhow, ensure, Result};
use bitcoin::{
    consensus::encode::{deserialize_hex, serialize_hex},
    hashes::Hash,
    Amount, FeeRate, Transaction,
};
use charms::{app, spell, spell::Spell, tx::add_spell, SPELL_VK};
use charms_data::{TxId, VkHash};
use std::{collections::BTreeMap, str::FromStr};

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
        funding_utxo_id,
        funding_utxo_value,
        change_address,
        fee_rate,
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

    // ciborium::into_writer(&(&norm_spell, &proof), std::io::stdout())?;

    // Serialize spell into CBOR
    let mut spell_data = vec![];
    ciborium::ser::into_writer(&(&norm_spell, &proof), &mut spell_data)?;

    // Parse funding UTXO
    let funding_utxo = crate::commands::tx::parse_outpoint(&funding_utxo_id)?;

    // Parse amount
    let funding_utxo_value = Amount::from_sat(funding_utxo_value);

    // Parse change address into ScriptPubkey
    let change_script_pubkey = bitcoin::Address::from_str(&change_address)?
        .assume_checked()
        .script_pubkey();

    // Parse fee rate
    let fee_rate = FeeRate::from_sat_per_kwu((fee_rate * 1000.0 / 4.0) as u64);

    // Call the add_spell function
    let transactions = add_spell(
        tx,
        &spell_data,
        funding_utxo,
        funding_utxo_value,
        change_script_pubkey,
        fee_rate,
    );

    // Convert transactions to hex and create JSON array
    let hex_txs: Vec<String> = transactions.iter().map(|tx| serialize_hex(tx)).collect();

    // Print JSON array of transaction hexes
    println!("{}", serde_json::to_string(&hex_txs)?);

    Ok(())
}
