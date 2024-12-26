pub mod app;
pub mod server;
pub mod spell;
pub mod tx;

use clap::{Args, Parser, Subcommand};
use std::{net::IpAddr, path::PathBuf};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Args)]
pub struct ServerConfig {
    /// IP address to listen on, defaults to 0.0.0.0 (all)
    #[arg(long, default_value = "0.0.0.0")]
    ip_addr: IpAddr,

    /// Port to listen on, defaults to 17784
    #[arg(long, default_value = "17784")]
    port: u16,

    /// bitcoind RPC URL. Set via RPC_URL env var.
    #[arg(long, env)]
    rpc_url: String,

    /// bitcoind RPC user. Recommended to set via RPC_USER env var.
    #[arg(long, env, default_value = "__cookie__")]
    rpc_user: String,

    /// bitcoind RPC password. Recommended to set via RPC_PASSWORD env var.
    /// Use the .cookie file in the bitcoind data directory to look up the password:
    /// the format is `__cookie__:password`
    #[arg(long, env)]
    rpc_password: String,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Charms API Server
    Server(#[command(flatten)] ServerConfig),

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

#[derive(Args)]
pub struct ProveConfig {
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
}

#[derive(Subcommand)]
pub enum SpellCommands {
    Parse,
    Prove(#[command(flatten)] ProveConfig),
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
        path: Option<PathBuf>,
    },

    /// Test the app for a spell.
    Run {
        /// Path to spell source file (YAML/JSON)
        #[arg(long, default_value = "/dev/stdin")]
        spell: PathBuf,

        /// Path to the app's RISC-V binary
        path: Option<PathBuf>,
    },
}

pub async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Server(server_config) => server::server(server_config).await,
        Commands::Spell { command } => match command {
            SpellCommands::Parse => spell::spell_parse(),
            SpellCommands::Prove(prove_config) => spell::spell_prove(prove_config),
        },
        Commands::Tx { command } => match command {
            TxCommands::AddSpell { .. } => tx::tx_add_spell(command),
            TxCommands::ShowSpell { tx } => tx::tx_show_spell(tx),
        },
        Commands::App { command } => match command {
            AppCommands::New { name } => app::new(&name),
            AppCommands::Vk { path } => app::vk(path),
            AppCommands::Build => app::build(),
            AppCommands::Run { spell, path } => app::run(spell, path),
        },
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn dummy() {}
}
