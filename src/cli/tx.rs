use crate::{cli, tx};
use anyhow::{anyhow, Result};
use bitcoin::{
    consensus::encode::{deserialize_hex, serialize_hex},
    OutPoint, Transaction,
};
use std::process::Command;

pub(crate) fn parse_outpoint(s: &str) -> Result<OutPoint> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid UTXO format. Expected txid:vout"));
    }

    Ok(OutPoint::new(parts[0].parse()?, parts[1].parse()?))
}

pub fn tx_show_spell(tx: String, json: bool) -> Result<()> {
    let tx = deserialize_hex::<Transaction>(&tx)?;

    match tx::spell(&tx) {
        Some(spell) => cli::print_output(&spell, json)?,
        None => eprintln!("No spell found in the transaction"),
    }

    Ok(())
}

pub(crate) fn get_prev_txs(tx: &Transaction) -> Result<Vec<Transaction>> {
    let cmd_output = Command::new("bash")
        .args(&[
            "-c", format!("bitcoin-cli decoderawtransaction {} | jq -r '.vin[].txid' | sort | uniq | xargs -I {{}} bitcoin-cli getrawtransaction {{}} | paste -sd, -", serialize_hex(tx)).as_str()
        ])
        .output()?;
    String::from_utf8(cmd_output.stdout)?
        .split(',')
        .map(|s| Ok(deserialize_hex::<Transaction>(s.trim())?))
        .collect()
}
