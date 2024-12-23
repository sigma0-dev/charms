pub mod app;
pub mod spell;
pub mod tx;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

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

    /// App contract commands
    App {
        #[command(subcommand)]
        command: AppCommands,
    },
}

#[derive(Subcommand)]
pub enum SpellCommands {
    Parse,
    Print,
    Prove {
        #[arg(long, default_value = "/dev/stdin")]
        spell: PathBuf,

        #[arg(long)]
        tx: String,

        #[arg(long, value_delimiter = ',')]
        prev_txs: Vec<String>,

        #[arg(long, value_delimiter = ',')]
        app_bins: Vec<PathBuf>,

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
    ShowSpell {
        #[arg(long)]
        tx: String,
    },
}

#[derive(Subcommand)]
pub enum AppCommands {
    /// Create a new app
    New {
        /// Name of the app. Directory <NAME> will be created.
        name: String,
    },

    /// Build the app
    Build,

    /// Show verification key for an app
    Vk {
        /// Path to the app's RISC-V binary
        path: Option<String>,
    },

    /// Generate the app proof for a spell.
    Prove,
}
