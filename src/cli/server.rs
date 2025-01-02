use crate::{cli::ServerConfig, spell::Spell, tx::spell_and_proof};
use anyhow::Result;
use axum::{extract::Path, http::StatusCode, routing::MethodRouter, Json, Router};
use bitcoin::{consensus::encode::deserialize_hex, Transaction};
use bitcoincore_rpc::{jsonrpc::Error::Rpc, Auth, Client, RpcApi};
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::OnceLock};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Types
#[derive(Debug, Serialize, Deserialize)]
struct DecodeSpell {
    tx_hex: String,
}

static RPC: OnceLock<Client> = OnceLock::new();

pub async fn server(
    ServerConfig {
        ip_addr,
        port,
        rpc_url,
        rpc_user,
        rpc_password,
    }: ServerConfig,
) -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    RPC.set(bitcoind_client(rpc_url, rpc_user, rpc_password))
        .expect("Should set RPC client");

    // Build router
    let app = Router::new().route(
        "/spells/{txid}",
        MethodRouter::new()
            .get(get_spell_handler)
            .put(put_spell_handler),
    );

    // Run server
    let addr = format!("{}:{}", ip_addr, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Server running on {}", &addr);

    axum::serve(listener, app).await?;
    Ok(())
}

// Handlers
async fn get_spell_handler(Path(txid): Path<String>) -> Result<Json<Spell>, StatusCode> {
    get_spell(&txid).map(Json)
}

async fn put_spell_handler(
    Path(txid): Path<String>,
    Json(payload): Json<DecodeSpell>,
) -> Result<Json<Spell>, StatusCode> {
    decode_spell(&txid, &payload).map(Json)
}

fn bitcoind_client(rpc_url: String, rpc_user: String, rpc_password: String) -> Client {
    Client::new(
        &rpc_url,
        Auth::UserPass(rpc_user.clone(), rpc_password.clone()),
    )
    .expect("Should create RPC client")
}

fn get_spell(txid: &str) -> Result<Spell, StatusCode> {
    let txid = bitcoin::Txid::from_str(txid).map_err(|_| StatusCode::BAD_REQUEST)?;

    let rpc = RPC.get().expect("RPC client should be initialized by now");
    match rpc.get_raw_transaction(&txid, None) {
        Ok(tx) => extract_spell(&tx),
        Err(e) => match e {
            bitcoincore_rpc::Error::JsonRpc(Rpc(rpc_error)) if rpc_error.code == -5 => {
                Err(StatusCode::NOT_FOUND)
            }
            _ => {
                eprintln!("Error: {:?}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        },
    }
}

fn decode_spell(txid: &str, request: &DecodeSpell) -> Result<Spell, StatusCode> {
    let txid = bitcoin::Txid::from_str(txid).map_err(|_| StatusCode::BAD_REQUEST)?;
    let tx: Transaction = deserialize_hex(&request.tx_hex).map_err(|_| StatusCode::BAD_REQUEST)?;
    if tx.compute_txid() != txid {
        return Err(StatusCode::BAD_REQUEST);
    }
    extract_spell(&tx)
}

fn extract_spell(tx: &Transaction) -> Result<Spell, StatusCode> {
    match spell_and_proof(&tx) {
        None => Err(StatusCode::NO_CONTENT),
        Some((spell, _)) => Ok(Spell::denormalized(&spell)),
    }
}
