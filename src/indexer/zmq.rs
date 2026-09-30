use bitcoin::Network;
use colored::Colorize;
use sqlx::PgPool;
use tokio::time::{Duration, interval};
use tracing::error;

use crate::db::blocks::insert_block;
use crate::db::state::set_last_indexed_height;
use crate::indexer::display::print_block;
use crate::rpc::client::RpcClient;

/// Polls Bitcoin Core every 5 seconds for new blocks and indexes them.
/// Runs forever until the process is killed.
pub async fn listen(pool: &PgPool, rpc: &RpcClient, network: Network) {
    println!(
        "\n{} Watching for new blocks (polling every 5s)...\n",
        "⟳".cyan().bold()
    );

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
                Ok(_) => {}
                Err(e) => {
                    error!("Failed to index block {height}: {e}");
                    break;
                }
            }
        }
    }
}

/// Fetches, indexes, and prints a new block with full detail.
async fn index_new_block(
    pool: &PgPool,
    rpc: &RpcClient,
    height: u64,
    network: Network,
) -> Result<(), Box<dyn std::error::Error>> {
    let hash = rpc.get_block_hash(height)?;
    let block = rpc.get_block(&hash)?;

    print_block(&block, height, network);

    insert_block(pool, &block, height as i32, network).await?;
    set_last_indexed_height(pool, height as i32).await?;

    Ok(())
}
