pub mod app;
pub mod server;
pub mod spell;
pub mod tx;
pub mod wallet;

use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use serde::Serialize;
use std::{io, net::IpAddr, path::PathBuf};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Args)]
pub struct ServerConfig {
    /// IP address to listen on, defaults to 0.0.0.0 (all).
    #[arg(long, default_value = "0.0.0.0")]
    ip_addr: IpAddr,

    /// Port to listen on, defaults to 17784.
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
    /// the format is `__cookie__:password`.
    #[arg(long, env)]
    rpc_password: String,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Charms API Server.
    Server(#[command(flatten)] ServerConfig),

    /// Work with spells.
    Spell {
        #[command(subcommand)]
        command: SpellCommands,
    },

    /// Work with underlying blockchain transactions.
    Tx {
        #[command(subcommand)]
        command: TxCommands,
    },

    /// Manage apps.
    App {
        #[command(subcommand)]
        command: AppCommands,
    },

    /// Wallet commands.
    Wallet {
        #[command(subcommand)]
        command: WalletCommands,
    },

    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Args)]
pub struct SpellProveParams {
    /// Spell source file (YAML/JSON).
    #[arg(long, default_value = "/dev/stdin")]
    spell: PathBuf,

    /// Bitcoin transaction (hex-encoded).
    #[arg(long)]
    tx: String,

    /// Pre-requisite transactions (hex-encoded) separated by commas (`,`).
    /// These are the transactions that create the UTXOs that the `tx` (and the spell) spends.
    /// If the spell has any reference UTXOs, the transactions creating them must also be included.
    #[arg(long, value_delimiter = ',')]
    prev_txs: Vec<String>,

    /// Path to the app binaries (RISC-V ELF files) referenced by the spell.
    #[arg(long, value_delimiter = ',')]
    app_bins: Vec<PathBuf>,

    /// UTXO ID of the funding transaction output (txid:vout).
    /// This UTXO will be spent to pay the fees (at the `fee-rate` per vB) for the commit and spell
    /// transactions. The rest of the value will be returned to the `change-address`.
    #[arg(long)]
    funding_utxo_id: String,
    /// Value of the funding UTXO in sats.
    #[arg(long)]
    funding_utxo_value: u64,

    /// Address to send the change to.
    #[arg(long)]
    change_address: String,

    /// Fee rate in sats/vB.
    #[arg(long, default_value = "2.0")]
    fee_rate: f64,
}

#[derive(Subcommand)]
pub enum SpellCommands {
    /// Prove a spell.
    Prove(#[command(flatten)] SpellProveParams),
}

#[derive(Subcommand)]
pub enum TxCommands {
    /// Show the spell in a transaction. If the transaction has a spell and its valid proof, it
    /// will be printed to stdout.
    ShowSpell {
        /// Hex-encoded transaction.
        #[arg(long)]
        tx: String,
        /// Output in JSON format (default is YAML).
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum AppCommands {
    /// Create a new app.
    New {
        /// Name of the app. Directory <NAME> will be created.
        name: String,
    },

    /// Build the app.
    Build,

    /// Show verification key for an app.
    Vk {
        /// Path to the app's RISC-V binary.
        path: Option<PathBuf>,
    },

    /// Test the app for a spell.
    Run {
        /// Path to spell source file (YAML/JSON).
        #[arg(long, default_value = "/dev/stdin")]
        spell: PathBuf,

        /// Path to the app's RISC-V binary.
        path: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum WalletCommands {
    /// List outputs with charms in the user's wallet.
    List(#[command(flatten)] WalletListParams),
    /// Cast a spell.
    /// Creates a spell, creates the underlying Bitcoin transaction, proves the spell, creates the
    /// commit transaction. Signs both the commit and spell transactions with the user's wallet.
    /// Returns the hex-encoded signed commit and spell transactions.
    Cast(#[command(flatten)] WalletCastParams),
}

#[derive(Args)]
pub struct WalletListParams {
    /// Output in JSON format (default is YAML)
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
pub struct WalletCastParams {
    /// Path to spell source file (YAML/JSON).
    #[arg(long, default_value = "/dev/stdin")]
    spell: PathBuf,
    /// Path to the apps' RISC-V binaries.
    #[arg(long, value_delimiter = ',')]
    app_bins: Vec<PathBuf>,
    /// Funding UTXO ID (`txid:vout`).
    #[arg(long)]
    funding_utxo_id: String,
    /// Fee rate in sats/vB.
    #[arg(long, default_value = "2.0")]
    fee_rate: f64,
}

pub async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Server(server_config) => server::server(server_config).await,
        Commands::Spell { command } => match command {
            SpellCommands::Prove(params) => spell::prove(params),
        },
        Commands::Tx { command } => match command {
            TxCommands::ShowSpell { tx, json } => tx::tx_show_spell(tx, json),
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
        Commands::Completions { shell } => generate_completions(shell),
    }
}

fn generate_completions(shell: Shell) -> anyhow::Result<()> {
    let cmd = &mut Cli::command();
    generate(shell, cmd, cmd.get_name().to_string(), &mut io::stdout());
    Ok(())
}

fn print_output<T: Serialize>(output: &T, json: bool) -> anyhow::Result<()> {
    match json {
        true => serde_json::to_writer_pretty(std::io::stdout(), &output)?,
        false => serde_yaml::to_writer(std::io::stdout(), &output)?,
    };
    Ok(())
}

#[cfg(test)]
mod test {
    #[test]
    fn dummy() {}
}
