use std::sync::{Arc, Mutex};
use tokio::time::{Duration, interval};

use crate::rpc::client::RpcClient;
use crate::tui::SharedState;
use crate::tui::app::NetworkStats;

pub async fn poll(rpc: &RpcClient, shared: Arc<Mutex<SharedState>>) {
    let mut ticker = interval(Duration::from_secs(30));

    loop {
        ticker.tick().await;

        if let Some(stats) = fetch_stats(rpc).await {
            if let Ok(mut state) = shared.lock() {
                state.network_stats = Some(stats);
            }
        }
    }
}

async fn fetch_stats(rpc: &RpcClient) -> Option<NetworkStats> {
    let chain_info = rpc.get_blockchain_info().await.ok()?;
    let mining_info = rpc.get_mining_info().await.ok()?;
    let hashps = rpc.get_network_hashps(120).await.ok()?;

    let height = chain_info.blocks;
    let next_adjustment_blocks = 2016 - (height as i64 % 2016);
    let halvings = height / 210_000;
    let block_subsidy_sats = if halvings >= 64 { 0 } else { 5_000_000_000u64 >> halvings };

    Some(NetworkStats {
        height,
        difficulty: chain_info.difficulty,
        network_hashps: hashps,
        mempool_tx_count: mining_info.pooled_tx,
        next_adjustment_blocks,
        last_block_time: chain_info.median_time,
        avg_block_time_secs: 600.0,
        block_subsidy_sats,
        chain: mining_info.chain,
    })
}
