mod config;
mod errors;
mod rpc;
mod indexer;
mod db;
mod api;
mod tui;

use bitcoin::Network;
use colored::Colorize;
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;

use config::Config;
use rpc::client::RpcClient;

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = Config::from_env();

    let network = match config.network.as_str() {
        "mainnet" => Network::Bitcoin,
        "testnet" => Network::Testnet,
        _ => Network::Regtest,
    };

    // --- Connect to Bitcoin Core ---
    println!("{}", "─".repeat(72).dimmed());
    println!(
        "  {} Connecting to Bitcoin Core at {}",
        "⟳".cyan(),
        config.rpc_url.yellow()
    );

    let rpc = RpcClient::new(&config).expect("Failed to connect to Bitcoin Core RPC");

    let chain_info = rpc
        .get_blockchain_info()
        .expect("Failed to get blockchain info");

    println!(
        "  {} Connected — chain: {}  height: {}",
        "✔".green().bold(),
        chain_info.chain.to_string().yellow(),
        chain_info.blocks.to_string().yellow()
    );

    // --- Connect to PostgreSQL ---
    println!("  {} Connecting to PostgreSQL...", "⟳".cyan());

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    println!("  {} Connected to PostgreSQL", "✔".green().bold());
    println!("{}\n", "─".repeat(72).dimmed());

    // --- Spawn the initial sync in the background ---
    let pool_sync = pool.clone();
    let rpc_sync = RpcClient::new(&config).expect("Failed to create sync RPC client");
    tokio::spawn(async move {
        indexer::runner::run(&pool_sync, &rpc_sync, network).await;
    });

    // --- Spawn the poller in the background ---
    let pool_poll = pool.clone();
    let rpc_poll = RpcClient::new(&config).expect("Failed to create poller RPC client");
    tokio::spawn(async move {
        indexer::zmq::listen(&pool_poll, &rpc_poll, network).await;
    });

    // --- Launch the TUI immediately ---
    tui::run(&pool).await.expect("TUI error");
}
