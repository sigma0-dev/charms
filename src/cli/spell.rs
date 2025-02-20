use crate::{cli::SpellProveParams, spell, spell::Spell, tx, tx::txs_by_txid, utils};
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
    let funding_utxo = crate::cli::tx::parse_outpoint(&funding_utxo_id)?;

    ensure!(fee_rate >= 1.0, "fee rate must be >= 1.0");

    let spell: Spell = serde_yaml::from_slice(&std::fs::read(spell)?)?;

    let tx = match tx {
        Some(tx) => deserialize_hex::<Transaction>(&tx)?,
        None => tx::from_spell(&spell),
    };
    let prev_txs = txs_by_txid(prev_txs)?;
    ensure!(tx
        .input
        .iter()
        .all(|input| prev_txs.contains_key(&input.previous_output.txid)));

    let transactions = spell::prove_spell_tx(
        spell,
        tx,
        app_bins,
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
