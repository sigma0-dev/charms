use anyhow::{anyhow, Result};
use bitcoin::{
    consensus::encode::{deserialize_hex, serialize_hex},
    Amount, FeeRate, OutPoint, ScriptBuf, Transaction,
};
use charms::spells::{add_spell, Spell};
use clap::{Parser, Subcommand};
use std::{io::Read, str::FromStr};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    AddSpell {
        #[arg(long)]
        tx: String,
        #[arg(long)]
        funding_utxo_id: String,
        #[arg(long)]
        funding_utxo_value: u64,
        #[arg(long)]
        change_address: String,
        #[arg(long)]
        fee_rate: f64,
    },
}

fn parse_outpoint(s: &str) -> Result<OutPoint> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid UTXO format. Expected txid:vout"));
    }

    Ok(OutPoint::new(parts[0].parse()?, parts[1].parse()?))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::AddSpell {
            tx,
            funding_utxo_id,
            funding_utxo_value,
            change_address,
            fee_rate,
        } => {
            // Read spell data from stdin
            let mut spell_data = Vec::new();
            std::io::stdin().read_to_end(&mut spell_data)?;

            // Deserialize spell using postcard
            let spell: Spell = postcard::from_bytes(&spell_data)?;

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
                &spell,
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
    }
}
