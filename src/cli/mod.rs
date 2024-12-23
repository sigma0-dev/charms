pub mod app;
pub mod server;
pub mod spell;
pub mod tx;

use clap::{Parser, Subcommand};
use std::{net::IpAddr, path::PathBuf};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Charms API Server
    Server {
        #[arg(long, default_value = "127.0.0.1")]
        ip_addr: IpAddr,

        #[arg(long, default_value = "3000")]
        port: u16,
    },

    /// Work with spells
    Spell {
        #[command(subcommand)]
        command: SpellCommands,
    },

    /// Low level transaction-related cli
    Tx {
        #[command(subcommand)]
        command: TxCommands,
    },

    /// Manage apps
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

pub async fn run() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Server { ip_addr, port } => server::server(ip_addr, port).await,
        Commands::Spell { command } => match command {
            SpellCommands::Parse => spell::spell_parse(),
            SpellCommands::Print => spell::spell_print(),
            SpellCommands::Prove { .. } => spell::spell_prove(command),
        },
        Commands::Tx { command } => match command {
            TxCommands::AddSpell { .. } => tx::tx_add_spell(command),
            TxCommands::ShowSpell { tx } => tx::tx_show_spell(tx),
        },
        Commands::App { command } => match command {
            AppCommands::New { name } => app::new(&name),
            AppCommands::Vk { path } => app::vk(path),
            AppCommands::Build => app::build(),
            AppCommands::Prove => {
                todo!()
            }
        },
    }
    .expect("Error");
}

#[cfg(test)]
mod test {
    #[test]
    fn dummy() {}
}
