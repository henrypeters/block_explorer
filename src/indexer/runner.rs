use bitcoin::Network;
use colored::Colorize;
use sqlx::PgPool;
use tracing::error;

use crate::db::blocks::insert_block;
use crate::db::state::{get_last_indexed_height, set_last_indexed_height};
use crate::indexer::display::print_block;
use crate::rpc::client::RpcClient;

/// Runs the initial sync — indexes all blocks from last indexed height to chain tip.
pub async fn run(pool: &PgPool, rpc: &RpcClient, network: Network) {
    let last_height = match get_last_indexed_height(pool).await {
        Ok(h) => h,
        Err(e) => {
            error!("Failed to read indexer state: {e}");
            return;
        }
    };

    let start_height = (last_height + 1) as u64;

    let chain_tip = match rpc.get_block_count() {
        Ok(h) => h,
        Err(e) => {
            error!("Failed to get block count from Bitcoin Core: {e}");
            return;
        }
    };

    if start_height > chain_tip {
        // Already up to date — re-fetch and print all indexed blocks for display
        println!(
            "\n{} Already up to date at height {}. Printing indexed blocks...\n",
            "✔".green().bold(),
            chain_tip.to_string().yellow()
        );
        print_all_indexed_blocks(pool, rpc, network).await;
        return;
    }

    println!(
        "\n{} Syncing blocks {} → {}\n",
        "⟳".cyan().bold(),
        start_height.to_string().yellow(),
        chain_tip.to_string().yellow()
    );

    for height in start_height..=chain_tip {
        match fetch_and_index(pool, rpc, height, network).await {
            Ok(_) => {}
            Err(e) => {
                error!("Failed to index block {height}: {e}");
                return;
            }
        }
    }

    println!(
        "\n{} Sync complete. Indexed up to block {}.\n",
        "✔".green().bold(),
        chain_tip.to_string().yellow()
    );
}

/// Silent version of run — used when TUI is active.
/// Indexes blocks without printing anything to stdout.
pub async fn run_silent(pool: &PgPool, rpc: &RpcClient, network: Network) {
    let last_height = match get_last_indexed_height(pool).await {
        Ok(h) => h,
        Err(e) => { error!("Failed to read indexer state: {e}"); return; }
    };

    let start_height = (last_height + 1) as u64;

    let chain_tip = match rpc.get_block_count() {
        Ok(h) => h,
        Err(e) => { error!("Failed to get block count: {e}"); return; }
    };

    if start_height > chain_tip {
        return;
    }

    for height in start_height..=chain_tip {
        if let Err(e) = fetch_and_index(pool, rpc, height, network).await {
            error!("Failed to index block {height}: {e}");
            return;
        }
    }
}
async fn print_all_indexed_blocks(pool: &PgPool, rpc: &RpcClient, network: Network) {
    let rows = match sqlx::query!("SELECT height FROM blocks ORDER BY height ASC")
        .fetch_all(pool)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to query indexed blocks: {e}");
            return;
        }
    };

    for row in rows {
        let height = row.height as u64;
        let hash = match rpc.get_block_hash(height) {
            Ok(h) => h,
            Err(e) => { error!("Failed to get hash for block {height}: {e}"); continue; }
        };
        let block = match rpc.get_block(&hash) {
            Ok(b) => b,
            Err(e) => { error!("Failed to fetch block {height}: {e}"); continue; }
        };
        print_block(&block, height, network);
    }
}

/// Fetches a block, writes it to the database, and prints its full detail.
pub async fn fetch_and_index(
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
