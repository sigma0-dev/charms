pub mod app;
pub mod server;
pub mod spell;
pub mod tx;
pub mod wallet;

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

    /// Wallet commands
    Wallet {
        #[command(subcommand)]
        command: WalletCommands,
    },
}

#[derive(Args)]
pub struct SpellProveParams {
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

#[derive(Args)]
pub struct SpellRenderParams {
    #[arg(long, default_value = "/dev/stdin")]
    formula: PathBuf,

    #[arg(long)]
    tx: String,

    #[arg(long, value_delimiter = ',')]
    prev_txs: Vec<String>,

    #[arg(long, value_delimiter = ',')]
    app_vks: Vec<String>,
}

#[derive(Subcommand)]
pub enum SpellCommands {
    Prove(#[command(flatten)] SpellProveParams),
    Render(#[command(flatten)] SpellRenderParams),
}

#[derive(Args)]
pub struct TxAddSpellParams {
    #[arg(long)]
    tx: String,
    #[arg(long, value_delimiter = ',')]
    prev_txs: Vec<String>,
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
pub enum TxCommands {
    AddSpell(#[command(flatten)] TxAddSpellParams),
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

#[derive(Subcommand)]
pub enum WalletCommands {
    /// List outputs with charms
    List(#[command(flatten)] WalletListParams),
    Cast(#[command(flatten)] WalletCastParams),
}

#[derive(Args)]
pub struct WalletListParams {
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
pub struct WalletCastParams {
    /// Path to spell source file (YAML/JSON)
    #[arg(long, default_value = "/dev/stdin")]
    spell: PathBuf,
    #[arg(long, value_delimiter = ',')]
    app_bins: Vec<PathBuf>,
    #[arg(long)]
    funding_utxo_id: String,
    #[arg(long, default_value = "2.0")]
    fee_rate: f64,
}

pub async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Server(server_config) => server::server(server_config).await,
        Commands::Spell { command } => match command {
            SpellCommands::Prove(params) => spell::prove(params),
            SpellCommands::Render(params) => spell::render(params),
        },
        Commands::Tx { command } => match command {
            TxCommands::AddSpell(params) => tx::tx_add_spell(params),
            TxCommands::ShowSpell { tx } => tx::tx_show_spell(tx),
        },
        Commands::App { command } => match command {
            AppCommands::New { name } => app::new(&name),
            AppCommands::Vk { path } => app::vk(path),
            AppCommands::Build => app::build(),
            AppCommands::Run { spell, path } => app::run(spell, path),
        },
        Commands::Wallet { command } => match command {
            WalletCommands::List(params) => wallet::list(params),
            WalletCommands::Cast(params) => wallet::cast(params),
        },
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn dummy() {}
}
