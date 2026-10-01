use bitcoin::Network;
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, interval};
use tracing::error;

use crate::db::blocks::insert_block;
use crate::db::state::set_last_indexed_height;
use crate::rpc::client::RpcClient;
use crate::tui::SharedState;

/// Polls Bitcoin Core every 5 seconds for new blocks and indexes them.
/// Updates shared state so the TUI can animate new block arrivals.
pub async fn listen_with_shared(
    pool: &PgPool,
    rpc: &RpcClient,
    network: Network,
    shared: Arc<Mutex<SharedState>>,
) {
    let mut ticker = interval(Duration::from_secs(5));

    loop {
        ticker.tick().await;

        let last_height = match crate::db::state::get_last_indexed_height(pool).await {
            Ok(h) => h,
            Err(e) => { error!("Failed to read indexer state: {e}"); continue; }
        };

        let chain_tip = match rpc.get_block_count() {
            Ok(h) => h as i32,
            Err(e) => { error!("Failed to get block count: {e}"); continue; }
        };

        if last_height >= chain_tip {
            continue;
        }

        for height in (last_height + 1)..=chain_tip {
            match index_new_block(pool, rpc, height as u64, network).await {
                Ok(_) => {
                    if let Ok(mut state) = shared.lock() {
                        state.latest_block_height = Some(height);
                    }
                }
                Err(e) => {
                    error!("Failed to index block {height}: {e}");
                    break;
                }
            }
        }
    }
}

/// Compatibility wrapper — used when no shared state is needed
pub async fn listen(pool: &PgPool, rpc: &RpcClient, network: Network) {
    listen_with_shared(
        pool,
        rpc,
        network,
        Arc::new(Mutex::new(SharedState {
            syncing: false,
            latest_block_height: None,
        })),
    )
    .await;
}

/// Fetches and indexes a single block silently.
async fn index_new_block(
    pool: &PgPool,
    rpc: &RpcClient,
    height: u64,
    network: Network,
) -> Result<(), Box<dyn std::error::Error>> {
    let hash = rpc.get_block_hash(height)?;
    let block = rpc.get_block(&hash)?;

    insert_block(pool, &block, height as i32, network).await?;
    set_last_indexed_height(pool, height as i32).await?;

    Ok(())
}
