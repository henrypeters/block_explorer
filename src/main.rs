mod config;
mod errors;
mod rpc;
mod indexer;
mod db;
mod api;
mod tui;

use bitcoin::Network;
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::sync::{Arc, Mutex};

use config::Config;
use rpc::client::RpcClient;
use tui::SharedState;

#[tokio::main]
async fn main() {
    dotenv().ok();

    // Disable all log output — the TUI owns the terminal completely.
    // Any output to stdout or stderr will corrupt the TUI display.
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::ERROR)
        .with_writer(std::io::sink)
        .init();

    let config = Config::from_env();

    let network = match config.network.as_str() {
        "mainnet" => Network::Bitcoin,
        "testnet" => Network::Testnet,
        _ => Network::Regtest,
    };

    // Connect to Bitcoin Core
    let rpc = RpcClient::new(&config).expect("Failed to connect to Bitcoin Core RPC");
    rpc.get_blockchain_info().expect("Failed to connect to Bitcoin Core");

    // Connect to PostgreSQL
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    // Shared state between background tasks and TUI
    let shared = Arc::new(Mutex::new(SharedState {
        syncing: true,
        latest_block_height: None,
        new_mempool_txs: Vec::new(),
        all_mempool_txs: Vec::new(),
    }));

    // --- Spawn initial sync ---
    let pool_sync = pool.clone();
    let rpc_sync = RpcClient::new(&config).expect("Failed to create sync RPC client");
    let shared_sync = shared.clone();
    tokio::spawn(async move {
        indexer::runner::run_silent(&pool_sync, &rpc_sync, network).await;
        if let Ok(mut state) = shared_sync.lock() {
            state.syncing = false;
        }
    });

    // --- Spawn block poller ---
    let pool_poll = pool.clone();
    let rpc_poll = RpcClient::new(&config).expect("Failed to create poller RPC client");
    let shared_poll = shared.clone();
    tokio::spawn(async move {
        indexer::zmq::listen_with_shared(&pool_poll, &rpc_poll, network, shared_poll).await;
    });

    // --- Spawn mempool poller ---
    let rpc_mempool = RpcClient::new(&config).expect("Failed to create mempool RPC client");
    let shared_mempool = shared.clone();
    tokio::spawn(async move {
        indexer::mempool::poll(&rpc_mempool, shared_mempool).await;
    });

    // --- Launch TUI ---
    tui::run(&pool, shared).await.expect("TUI error");
}
