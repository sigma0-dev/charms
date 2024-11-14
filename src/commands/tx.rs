use crate::commands::TxCommands;
use anyhow::{anyhow, bail, ensure, Result};
use bitcoin::{
    consensus::encode::{deserialize_hex, serialize_hex},
    opcodes::all::OP_IF,
    script::{Instruction, PushBytes},
    Amount, FeeRate, OutPoint, Transaction,
};
use charms::{spell::Spell, tx::add_spell};
use std::str::FromStr;

fn parse_outpoint(s: &str) -> Result<OutPoint> {
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
    let spell: Spell = ciborium::de::from_reader(std::io::stdin())?;

    // Serialize spell into CBOR
    let mut spell_data = vec![];
    ciborium::ser::into_writer(&spell, &mut spell_data)?;

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

pub(crate) fn tx_extract_spell(command: TxCommands) -> Result<()> {
    let TxCommands::ExtractSpell { tx } = command else {
        unreachable!()
    };
    let tx = deserialize_hex::<Transaction>(&tx)?;

    let script_data = &tx.input[tx.input.len() - 1].witness[1];

    // Parse script_data into Script
    let script = bitcoin::blockdata::script::Script::from_bytes(&script_data);

    let mut instructions = script.instructions();

    ensure!(instructions.next() == Some(Ok(Instruction::PushBytes(PushBytes::empty()))));
    ensure!(instructions.next() == Some(Ok(Instruction::Op(OP_IF))));
    let Some(Ok(Instruction::PushBytes(push_bytes))) = instructions.next() else {
        bail!("no spell")
    };
    if push_bytes.as_bytes() != b"spell" {
        bail!("no spell")
    }
    let Some(Ok(Instruction::PushBytes(push_bytes))) = instructions.next() else {
        bail!("no spell")
    };

    let spell_data = push_bytes.as_bytes();
    let spell: Spell = ciborium::de::from_reader(spell_data)?;

    ciborium::into_writer(&spell, std::io::stdout())?;

    Ok(())
}
