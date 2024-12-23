use crate::{spell::Spell, tx::spell_and_proof};
use anyhow::Result;
use axum::{http::StatusCode, routing::get, Json, Router};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use serde::{Deserialize, Serialize};
use std::{net::IpAddr, path::PathBuf, str::FromStr, sync::LazyLock};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Types
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Item {
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateItem {
    name: String,
    description: Option<String>,
}

pub async fn server(ip_addr: IpAddr, port: u16) -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Build router
    let app = Router::new().route("/spells/{txid}", get(get_item));

    // Run server
    let addr = format!("{}:{}", ip_addr, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Server running on {}", &addr);

    axum::serve(listener, app).await?;
    Ok(())
}

// Handlers
async fn get_item(
    axum::extract::Path(txid): axum::extract::Path<String>,
) -> Result<Json<Spell>, StatusCode> {
    get_spell(&txid).map(Json).ok_or(StatusCode::NOT_FOUND)
}

static RPC: LazyLock<Client> = LazyLock::new(|| {
    // Configure the RPC connection
    let rpc_url = "http://127.0.0.1:48332";
    // let rpc_user = "";
    // let rpc_password = "";

    // Create RPC client
    let rpc = Client::new(
        rpc_url,
        Auth::CookieFile(PathBuf::from(format!(
            "{}/Library/Application Support/Bitcoin/testnet4/.cookie",
            std::env::var("HOME").unwrap()
        ))),
        // Auth::UserPass(rpc_user.to_string(), rpc_password.to_string()),
    )
    .expect("Should connect to bitcoind");

    rpc
});

fn get_spell(txid: &str) -> Option<Spell> {
    let txid = bitcoin::Txid::from_str(txid).ok()?;

    match RPC.get_raw_transaction(&txid, None) {
        Ok(tx) => match spell_and_proof(&tx) {
            None => None,
            Some((s, _)) => Some(Spell::denormalized(&s)),
        },
        Err(e) => {
            eprintln!("Error fetching transaction: {}", e);
            None
        }
    }
}
