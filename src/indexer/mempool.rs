use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{Duration, interval};

use crate::rpc::client::RpcClient;
use crate::tui::SharedState;
use crate::tui::app::MempoolTx;

fn now_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn format_time(unix: u64) -> String {
    let secs = unix % 86400;
    format!("{:02}:{:02}:{:02}", secs / 3600, (secs % 3600) / 60, secs % 60)
}

pub async fn poll(rpc: &RpcClient, shared: Arc<Mutex<SharedState>>) {
    let mut ticker = interval(Duration::from_secs(3));
    let mut known_txids: HashSet<String> = HashSet::new();
    let start_time = now_unix();
    let mut first_poll = true;

    loop {
        ticker.tick().await;

        let txids = match rpc.get_raw_mempool().await {
            Ok(t) => t,
            Err(_) => continue,
        };

        let current_txids: HashSet<String> = txids.iter().map(|t| t.to_string()).collect();
        let mut all_txs: Vec<MempoolTx> = Vec::new();
        let mut new_txs: Vec<MempoolTx> = Vec::new();

        for txid in &txids {
            match rpc.get_mempool_entry(txid).await {
                Ok(entry) => {
                    let fee_sats = entry.fees.to_sat();
                    let txid_str = txid.to_string();
                    let is_brand_new = !known_txids.contains(&txid_str);
                    let notify = is_brand_new && !first_poll && entry.time >= start_time;

                    let tx = MempoolTx {
                        txid: txid_str,
                        fee_sats,
                        size: entry.vsize as u32,
                        time: entry.time,
                        arrived_at: format_time(now_unix()),
                        is_new: notify,
                    };

                    if notify { new_txs.push(tx.clone()); }
                    all_txs.push(tx);
                }
                Err(_) => {} // tx confirmed or evicted — skip silently
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
