mod config;
mod errors;
mod rpc;
mod indexer;
mod db;
mod api;

use bitcoin::Network;
use bitcoin::consensus::serialize;
use dotenvy::dotenv;
use tracing::info;

use config::Config;
use rpc::client::RpcClient;

fn main() {
    // Load .env file if present
    dotenv().ok();

    // Initialize logging — set RUST_LOG=info to see output
    tracing_subscriber::fmt::init();

    // Load config from environment
    let config = Config::from_env();
    info!("Connecting to Bitcoin Core at {}", config.rpc_url);
    info!("Network: {}", config.network);

    // Connect to Bitcoin Core
    let client = RpcClient::new(&config).expect("Failed to connect to Bitcoin Core RPC");  // This is where we create a connection

    // --- Chain info ---
    let chain_info = client  // we are using the connection here to make rpc call
        .get_blockchain_info()
        .expect("Failed to get blockchain info");

    println!("\n=== Chain Info ===");
    println!("Chain:       {}", chain_info.chain);
    println!("Height:      {}", chain_info.blocks);
    println!("Best block:  {}", chain_info.best_block_hash);

    // --- Fetch block 0 (genesis) ---
    for _block in 0..=chain_info.blocks{

        println!("\n=== Fetching block at height {_block} ===");
    
        let block_hash = client
            .get_block_hash(_block)
            .expect("Failed to get block hash");  // also using the connection to make an rpc call
    
        println!("Hash: {block_hash}");
    
        let block = client
            .get_block(&block_hash)
            .expect("Failed to fetch and decode block");
    
        // --- Print block summary ---
        println!("\n=== Block Summary ===");
        println!("Version:       {:?}", block.header.version);
        println!("Prev block:    {}", block.header.prev_blockhash);
        println!("Merkle root:   {}", block.header.merkle_root);
        println!("Timestamp:     {}", block.header.time);
        println!("Bits:          {:#010x}", block.header.bits.to_consensus());
        println!("Nonce:         {}", block.header.nonce);
        println!("Tx count:      {}", block.txdata.len());
        println!("Block size:    {} bytes", serialize(&block).len());
    
        // --- Print each transaction ---
        println!("\n=== Transactions ===");
        for (i, tx) in block.txdata.iter().enumerate() {
            let txid = tx.compute_txid();
            let is_coinbase = tx.is_coinbase();
    
            println!("\n  TX #{i}");
            println!("  TXID:        {txid}");
            println!("  Coinbase:    {is_coinbase}");
            println!("  Version:     {}", tx.version);
            println!("  Locktime:    {}", tx.lock_time);
            println!("  Inputs:      {}", tx.input.len());
            println!("  Outputs:     {}", tx.output.len());
    
            // Print inputs
            for (j, input) in tx.input.iter().enumerate() {
                if is_coinbase {
                    println!("    Input #{j}: coinbase");
                } else {
                    println!(
                        "    Input #{j}: {}:{}",
                        input.previous_output.txid, input.previous_output.vout
                    );
                }
            }
    
            // Print outputs
            let network = match config.network.as_str() {
                "mainnet" => Network::Bitcoin,
                "testnet" => Network::Testnet,
                _ => Network::Regtest,
            };
    
            for (k, output) in tx.output.iter().enumerate() {
                let value_sats = output.value.to_sat();
                let address = bitcoin::Address::from_script(&output.script_pubkey, network)
                    .map(|a| a.to_string())
                    .unwrap_or_else(|_| "undecodable script".to_string());
    
                println!("    Output #{k}: {value_sats} sats → {address}");
            }
        }
    }


    // println!("\nPhase 1 complete — raw block fetched and decoded successfully.");
}
