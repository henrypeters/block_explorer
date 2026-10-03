use bitcoin::Network;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{Duration, sleep};
use tracing::error;

use crate::db::blocks::insert_block;
use crate::db::state::{get_last_indexed_height, set_last_indexed_height};
use crate::config::Config;
use crate::rpc::client::RpcClient;

const MAX_CONCURRENT: usize = 8;
const MAX_RETRIES: u32 = 4;
const RETRY_DELAY_MS: u64 = 800;
const CHUNK_SIZE: u64 = 30;

/// Runs forever — syncs to chain tip then waits for new blocks.
/// Automatically resumes if a batch fails partway through.
pub async fn run_silent(pool: &PgPool, rpc: &RpcClient, network: Network) {
    // Fix any mismatch between indexer_state and actual blocks on startup
    let _ = sqlx::query!(
        "UPDATE indexer_state SET last_indexed_height = COALESCE((SELECT MAX(height) FROM blocks), -1)"
    )
    .execute(pool)
    .await;

    loop {
        let last_height = match get_last_indexed_height(pool).await {
            Ok(h) => h,
            Err(e) => { error!("Failed to read indexer state: {e}"); sleep(Duration::from_secs(5)).await; continue; }
        };

        let chain_tip = match rpc.get_block_count().await {
            Ok(h) => h,
            Err(e) => { error!("Failed to get block count: {e}"); sleep(Duration::from_secs(10)).await; continue; }
        };

        let start_height = (last_height + 1) as u64;

        if start_height > chain_tip {
            // Caught up — check again in 30 seconds
            sleep(Duration::from_secs(30)).await;
            continue;
        }

        index_range(pool, rpc, network, start_height, chain_tip).await;
    }
}

/// Indexes a range of blocks in chunks of CHUNK_SIZE with MAX_CONCURRENT parallelism.
async fn index_range(pool: &PgPool, rpc: &RpcClient, network: Network, start: u64, end: u64) {
    let rpc_url = rpc.url.clone();
    let rpc_user = rpc.user.clone();
    let rpc_pass = rpc.password.clone();
    let network_str = match network {
        Network::Bitcoin => "mainnet",
        Network::Testnet => "testnet",
        _ => "regtest",
    }.to_string();

    let mut height = start;

    while height <= end {
        let chunk_end = (height + CHUNK_SIZE - 1).min(end);
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT));
        let mut handles = Vec::new();

        for h in height..=chunk_end {
            let sem = semaphore.clone();
            let url = rpc_url.clone();
            let user = rpc_user.clone();
            let pass = rpc_pass.clone();
            let net = network_str.clone();

            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();

                for attempt in 0..MAX_RETRIES {
                    if attempt > 0 {
                        sleep(Duration::from_millis(RETRY_DELAY_MS * attempt as u64)).await;
                    }

                    let cfg = Config {
                        rpc_url: url.clone(),
                        rpc_user: user.clone(),
                        rpc_password: pass.clone(),
                        database_url: String::new(),
                        network: net.clone(),
                        zmq_block_url: String::new(),
                    };

                    let client = match RpcClient::new(&cfg) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };

                    let hash = match client.get_block_hash(h).await {
                        Ok(hash) => hash,
                        Err(_) => continue,
                    };

                    match client.get_block(&hash).await {
                        Ok(block) => return Some((h, block)),
                        Err(_) => continue,
                    }
                }

                error!("Skipping block {h} after {MAX_RETRIES} failed attempts");
                None
            });

            handles.push(handle);
        }

        for handle in handles {
            match handle.await {
                Ok(Some((h, block))) => {
                    if let Err(e) = insert_block(pool, &block, h as i32, network).await {
                        error!("Failed to insert block {h}: {e}");
                        continue;
                    }
                    if let Err(e) = set_last_indexed_height(pool, h as i32).await {
                        error!("Failed to update state at {h}: {e}");
                    }
                }
                Ok(None) => {}
                Err(e) => { error!("Task join error: {e}"); }
            }
        }

        // Always advance past this chunk — even if some blocks were skipped
        // This prevents getting stuck retrying the same failed blocks forever
        let _ = set_last_indexed_height(pool, chunk_end as i32).await;

        height = chunk_end + 1;
    }
}

pub async fn fetch_and_index(
    pool: &PgPool,
    rpc: &RpcClient,
    height: u64,
    network: Network,
) -> Result<(), Box<dyn std::error::Error>> {
    let hash = rpc.get_block_hash(height).await?;
    let block = rpc.get_block(&hash).await?;
    insert_block(pool, &block, height as i32, network).await?;
    set_last_indexed_height(pool, height as i32).await?;
    Ok(())
}
