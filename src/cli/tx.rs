use crate::{cli::TxCommands, spell::Spell, tx, tx::add_spell};
use anyhow::{anyhow, Result};
use bitcoin::{
    consensus::encode::{deserialize_hex, serialize_hex},
    Amount, FeeRate, OutPoint, Transaction,
};
use charms_spell_checker::{NormalizedSpell, Proof};
use std::str::FromStr;

pub(crate) fn parse_outpoint(s: &str) -> Result<OutPoint> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid UTXO format. Expected txid:vout"));
    }

    Ok(OutPoint::new(parts[0].parse()?, parts[1].parse()?))
}

pub fn tx_add_spell(command: TxCommands) -> Result<()> {
    let TxCommands::AddSpell {
        tx,
        funding_utxo_id,
        funding_utxo_value,
        change_address,
        fee_rate,
    } = command
    else {
        unreachable!()
    };

    // Read spell data from stdin
    let spell_and_proof: (NormalizedSpell, Proof) = ciborium::de::from_reader(std::io::stdin())?;

    // Serialize spell into CBOR
    let mut spell_data = vec![];
    ciborium::ser::into_writer(&spell_and_proof, &mut spell_data)?;

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

pub fn tx_show_spell(tx: String) -> Result<()> {
    let tx = deserialize_hex::<Transaction>(&tx)?;

    if let Some((spell, _)) = tx::spell_and_proof(&tx) {
        serde_yaml::to_writer(std::io::stdout(), &Spell::denormalized(&spell))?;
    } else {
        eprintln!("No spell found in the transaction");
    }

    Ok(())
}
