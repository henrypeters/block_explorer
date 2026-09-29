use serde::{Deserialize, Serialize};

/// A lightweight summary of a block, used for printing and later API responses.
#[derive(Debug, Serialize, Deserialize)]
pub struct BlockSummary {
    pub height: u64,
    pub hash: String,
    pub timestamp: u32,
    pub tx_count: usize,
    pub size: usize,
}

/// A lightweight summary of a transaction.
#[derive(Debug, Serialize, Deserialize)]
pub struct TxSummary {
    pub txid: String,
    pub input_count: usize,
    pub output_count: usize,
    pub is_coinbase: bool,
}
