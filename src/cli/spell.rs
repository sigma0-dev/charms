use crate::{
    app, cli,
    cli::{SpellCheckParams, SpellProveParams},
    spell,
    spell::Spell,
    tx,
    tx::txs_by_txid,
    utils, SPELL_VK,
};
use anyhow::{ensure, Result};
use bitcoin::{
    consensus::encode::{deserialize_hex, serialize_hex},
    Transaction,
};

pub fn prove(
    SpellProveParams {
        spell,
        tx,
        prev_txs,
        app_bins,
        funding_utxo_id,
        funding_utxo_value,
        change_address,
        fee_rate,
    }: SpellProveParams,
) -> Result<()> {
    utils::logger::setup_logger();

    // Parse funding UTXO early: to fail fast
    let funding_utxo = cli::tx::parse_outpoint(&funding_utxo_id)?;

    ensure!(fee_rate >= 1.0, "fee rate must be >= 1.0");

    let spell: Spell = serde_yaml::from_slice(&std::fs::read(spell)?)?;

    let tx = match tx {
        Some(tx) => deserialize_hex::<Transaction>(&tx)?,
        None => tx::from_spell(&spell),
    };
    let prev_txs = prev_txs
        .into_iter()
        .map(|tx| Ok(deserialize_hex::<Transaction>(&tx)?))
        .collect::<Result<_>>()?;
    let prev_txs = txs_by_txid(prev_txs)?;
    ensure!(tx
        .input
        .iter()
        .all(|input| prev_txs.contains_key(&input.previous_output.txid)));

    let app_prover = app::Prover::new();
    let binaries = cli::app::binaries_by_vk(&app_prover, app_bins)?;

    let transactions = spell::prove_spell_tx(
        spell,
        tx,
        binaries,
        prev_txs,
        funding_utxo,
        funding_utxo_value,
        change_address,
        fee_rate,
    )?;

    // Convert transactions to hex and create JSON array
    let hex_txs: Vec<String> = transactions.iter().map(|tx| serialize_hex(tx)).collect();

    // Print JSON array of transaction hexes
    println!("{}", serde_json::to_string(&hex_txs)?);

    Ok(())
}

pub fn check(SpellCheckParams { spell, app_bins }: SpellCheckParams) -> Result<()> {
    utils::logger::setup_logger();

    let mut spell: Spell = serde_yaml::from_slice(&std::fs::read(spell)?)?;
    for u in spell.outs.iter_mut() {
        u.sats.get_or_insert(crate::cli::wallet::MIN_SATS);
    }

    // make sure spell inputs all have utxo_id
    ensure!(
        spell.ins.iter().all(|u| u.utxo_id.is_some()),
        "all spell inputs must have utxo_id"
    );

    let tx = tx::from_spell(&spell);

    let prev_txs = cli::tx::get_prev_txs(&tx)?;

    eprintln!("checking prev_txs");
    let prev_spells = charms_client::prev_spells(&prev_txs, &SPELL_VK);
    eprintln!("checking prev_txs... done!");

    let (norm_spell, app_private_inputs) = spell.normalized()?;
    let norm_spell = spell::align_spell_to_tx(norm_spell, &tx)?;

    eprintln!("checking spell is well-formed");
    ensure!(
        charms_client::well_formed(&norm_spell, &prev_spells),
        "spell is not well-formed"
    );
    eprintln!("checking spell is well-formed... done!");

    eprintln!("checking spell is correct");
    let app_prover = app::Prover::new();

    let binaries = cli::app::binaries_by_vk(&app_prover, app_bins)?;

    let charms_tx = spell.to_tx()?;
    app_prover.run_all(
        &binaries,
        &charms_tx,
        &norm_spell.app_public_inputs,
        app_private_inputs,
    )?;
    eprintln!("checking spell is correct... done!");

    Ok(())
}
