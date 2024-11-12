pub mod spell;
pub mod tx;

use clap::{Parser, Subcommand};
use std::{io::Read, str::FromStr};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Spell {
        #[command(subcommand)]
        command: SpellCommands,
    },

    /// Low level transaction-related commands
    Tx {
        #[command(subcommand)]
        command: TxCommands,
    },
}

#[derive(Subcommand)]
pub enum SpellCommands {
    Parse,
    Print,
}

#[derive(Subcommand)]
pub enum TxCommands {
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
