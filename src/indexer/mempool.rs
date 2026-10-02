use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{Duration, interval};

use crate::rpc::client::RpcClient;
use crate::tui::SharedState;
use crate::tui::app::MempoolTx;

/// Returns current unix timestamp as u64.
fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Formats a unix timestamp as HH:MM:SS.
fn format_time(unix: u64) -> String {
    let secs = unix % 86400;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

/// Polls Bitcoin Core's mempool every 3 seconds.
/// Only notifies the TUI for transactions that arrived after the program started.
/// Pre-existing transactions are loaded silently into the mempool panel.
pub async fn poll(rpc: &RpcClient, shared: Arc<Mutex<SharedState>>) {
    let mut ticker = interval(Duration::from_secs(3));
    let mut known_txids: HashSet<String> = HashSet::new();
    let start_time = now_unix();
    let mut first_poll = true;

    loop {
        ticker.tick().await;

        let txids = match rpc.get_raw_mempool() {
            Ok(t) => t,
            Err(_) => continue,
        };

        let current_txids: HashSet<String> = txids.iter().map(|t| t.to_string()).collect();

        let mut all_txs: Vec<MempoolTx> = Vec::new();
        let mut new_txs: Vec<MempoolTx> = Vec::new();

        for txid in &txids {
            match rpc.get_mempool_entry(txid) {
                Ok(entry) => {
                    let fee_sats = entry.fees.base.to_sat();
                    let txid_str = txid.to_string();
                    let is_brand_new = !known_txids.contains(&txid_str);

                    // Only notify for txs that arrived after program start
                    // and not on the first poll (first poll = pre-existing txs)
                    let notify = is_brand_new && !first_poll && entry.time >= start_time;

                    let tx = MempoolTx {
                        txid: txid_str,
                        fee_sats,
                        size: entry.vsize as u32,
                        time: entry.time,
                        arrived_at: format_time(now_unix()),
                        is_new: notify,
                    };

                    if notify {
                        new_txs.push(tx.clone());
                    }
                    all_txs.push(tx);
                }
                Err(_) => {
                    // Transaction was confirmed or evicted between get_raw_mempool
                    // and get_mempool_entry — silently skip it, this is normal
                }
            }
        }

        known_txids = current_txids;
        first_poll = false;

        if let Ok(mut state) = shared.lock() {
            state.all_mempool_txs = all_txs;
            if !new_txs.is_empty() {
                state.new_mempool_txs.extend(new_txs);
            }
        }
    }
}
