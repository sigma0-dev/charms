use crate::{
    cli::TxAddSpellParams,
    tx,
    tx::{add_spell, txs_by_txid},
};
use anyhow::{anyhow, ensure, Result};
use bitcoin::{
    consensus::encode::{deserialize_hex, serialize_hex},
    Amount, FeeRate, OutPoint, Transaction,
};
use charms_data::util;
use charms_spell_checker::{NormalizedSpell, Proof};
use std::str::FromStr;

pub(crate) fn parse_outpoint(s: &str) -> Result<OutPoint> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid UTXO format. Expected txid:vout"));
    }

    Ok(OutPoint::new(parts[0].parse()?, parts[1].parse()?))
}

pub fn tx_add_spell(
    TxAddSpellParams {
        tx,
        prev_txs,
        funding_utxo_id,
        funding_utxo_value,
        change_address,
        fee_rate,
    }: TxAddSpellParams,
) -> Result<()> {
    // Read spell data from stdin
    let spell_and_proof: (NormalizedSpell, Proof) = util::read(std::io::stdin())?;

    // Serialize spell into CBOR
    let spell_data = util::write(&spell_and_proof)?;

    // Parse transaction from hex
    let tx = deserialize_hex::<Transaction>(&tx)?;

    // Parse funding UTXO
    let funding_utxo = parse_outpoint(&funding_utxo_id)?;

    // Parse amount
    let funding_utxo_value = Amount::from_sat(funding_utxo_value);

    // Parse change address into ScriptPubkey
    let change_script_pubkey = bitcoin::Address::from_str(&change_address)?
        .assume_checked()
        .script_pubkey();

    // Parse fee rate
    let fee_rate = FeeRate::from_sat_per_kwu((fee_rate * 1000.0 / 4.0) as u64);

    let prev_txs = txs_by_txid(prev_txs)?;
    ensure!(tx
        .input
        .iter()
        .all(|input| prev_txs.contains_key(&input.previous_output.txid)));

    // Call the add_spell function
    let transactions = add_spell(
        tx,
        &spell_data,
        funding_utxo,
        funding_utxo_value,
        change_script_pubkey,
        fee_rate,
        &prev_txs,
    );

    // Convert transactions to hex and create JSON array
    let hex_txs: Vec<String> = transactions.iter().map(|tx| serialize_hex(tx)).collect();

    // Print JSON array of transaction hexes
    println!("{}", serde_json::to_string(&hex_txs)?);
    Ok(())
}

pub fn tx_show_spell(tx: String) -> Result<()> {
    let tx = deserialize_hex::<Transaction>(&tx)?;

    match tx::spell(&tx) {
        Some(spell) => serde_yaml::to_writer(std::io::stdout(), &spell)?,
        None => eprintln!("No spell found in the transaction"),
    }

    Ok(())
}
