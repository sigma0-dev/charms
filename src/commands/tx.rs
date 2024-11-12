use crate::commands::TxCommands;
use anyhow::anyhow;
use bitcoin::{
    consensus::encode::{deserialize_hex, serialize_hex},
    Amount, FeeRate, OutPoint, Transaction,
};
use charms::{spell::Spell, tx::add_spell};
use std::{io::Read, str::FromStr};

fn parse_outpoint(s: &str) -> anyhow::Result<OutPoint> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid UTXO format. Expected txid:vout"));
    }

    Ok(OutPoint::new(parts[0].parse()?, parts[1].parse()?))
}

pub fn tx_add_spell(command: TxCommands) -> anyhow::Result<()> {
    let TxCommands::AddSpell {
        tx,
        funding_utxo_id,
        funding_utxo_value,
        change_address,
        fee_rate,
    } = command;

    // Read spell data from stdin
    let mut spell_data = Vec::new();
    std::io::stdin().read_to_end(&mut spell_data)?;

    // Parse spell using postcard
    let _: Spell = postcard::from_bytes(&spell_data)?;

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
