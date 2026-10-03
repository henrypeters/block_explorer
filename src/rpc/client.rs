use bitcoin::Block;
use bitcoin::consensus::deserialize;
use reqwest::Client;
use serde_json::{Value, json};

use crate::config::Config;
use crate::errors::ExplorerError;

/// Async JSON-RPC client compatible with both local Bitcoin Core
/// and remote providers like Chainstack.
pub struct RpcClient {
    pub url: String,
    pub user: String,
    pub password: String,
    http: Client,
}

impl RpcClient {
    pub fn new(config: &Config) -> Result<Self, ExplorerError> {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ExplorerError::Other(e.to_string()))?;

        Ok(Self {
            url: config.rpc_url.clone(),
            user: config.rpc_user.clone(),
            password: config.rpc_password.clone(),
            http,
        })
    }

    /// Send a JSON-RPC call and return the result field.
    async fn call(&self, method: &str, params: Value) -> Result<Value, ExplorerError> {
        let body = json!({
            "jsonrpc": "1.0",
            "id": "stratabtc",
            "method": method,
            "params": params
        });

        let resp = self.http
            .post(&self.url)
            .basic_auth(&self.user, Some(&self.password))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ExplorerError::Other(e.to_string()))?;

        let json: Value = resp.json()
            .await
            .map_err(|e| ExplorerError::Other(e.to_string()))?;

        if let Some(err) = json.get("error").filter(|e| !e.is_null()) {
            return Err(ExplorerError::Other(err.to_string()));
        }

        Ok(json["result"].clone())
    }

    pub async fn get_block_count(&self) -> Result<u64, ExplorerError> {
        let result = self.call("getblockcount", json!([])).await?;
        result.as_u64().ok_or_else(|| ExplorerError::Other("invalid blockcount".into()))
    }

    pub async fn get_block_hash(&self, height: u64) -> Result<bitcoin::BlockHash, ExplorerError> {
        let result = self.call("getblockhash", json!([height])).await?;
        let hash_str = result.as_str().ok_or_else(|| ExplorerError::Other("invalid hash".into()))?;
        hash_str.parse::<bitcoin::BlockHash>()
            .map_err(|e| ExplorerError::Other(e.to_string()))
    }

    pub async fn get_block(&self, hash: &bitcoin::BlockHash) -> Result<Block, ExplorerError> {
        let result = self.call("getblock", json!([hash.to_string(), 0])).await?;
        let hex = result.as_str().ok_or_else(|| ExplorerError::Other("invalid block hex".into()))?;
        let raw = hex::decode(hex)?;
        let block: Block = deserialize(&raw)?;
        Ok(block)
    }

    pub async fn get_blockchain_info(&self) -> Result<BlockchainInfo, ExplorerError> {
        let result = self.call("getblockchaininfo", json!([])).await?;
        serde_json::from_value(result).map_err(|e| ExplorerError::Other(e.to_string()))
    }

    pub async fn get_block_header_info(&self, hash: &bitcoin::BlockHash) -> Result<BlockHeaderInfo, ExplorerError> {
        let result = self.call("getblockheader", json!([hash.to_string(), true])).await?;
        serde_json::from_value(result).map_err(|e| ExplorerError::Other(e.to_string()))
    }

    pub async fn get_raw_mempool(&self) -> Result<Vec<bitcoin::Txid>, ExplorerError> {
        let result = self.call("getrawmempool", json!([false])).await?;
        let arr = result.as_array().ok_or_else(|| ExplorerError::Other("invalid mempool".into()))?;
        arr.iter()
            .map(|v| {
                v.as_str()
                    .ok_or_else(|| ExplorerError::Other("invalid txid".into()))
                    .and_then(|s| s.parse::<bitcoin::Txid>().map_err(|e| ExplorerError::Other(e.to_string())))
            })
            .collect()
    }

    pub async fn get_mempool_entry(&self, txid: &bitcoin::Txid) -> Result<MempoolEntry, ExplorerError> {
        let result = self.call("getmempoolentry", json!([txid.to_string()])).await?;
        serde_json::from_value(result).map_err(|e| ExplorerError::Other(e.to_string()))
    }

    pub async fn get_mining_info(&self) -> Result<MiningInfo, ExplorerError> {
        let result = self.call("getmininginfo", json!([])).await?;
        serde_json::from_value(result).map_err(|e| ExplorerError::Other(e.to_string()))
    }

    pub async fn get_network_hashps(&self, n_blocks: u32) -> Result<f64, ExplorerError> {
        let result = self.call("getnetworkhashps", json!([n_blocks, -1])).await?;
        result.as_f64().ok_or_else(|| ExplorerError::Other("invalid hashps".into()))
    }
}

// ─── Response types ──────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct BlockchainInfo {
    pub chain: String,
    pub blocks: u64,
    #[serde(rename = "bestblockhash")]
    pub best_block_hash: String,
    pub difficulty: f64,
    #[serde(rename = "mediantime")]
    pub median_time: u64,
}

#[derive(Debug, serde::Deserialize)]
pub struct BlockHeaderInfo {
    pub height: u64,
    pub hash: String,
    pub time: u64,
}

#[derive(Debug, serde::Deserialize)]
pub struct MempoolEntry {
    pub fees: MempoolFees,
    pub vsize: u64,
    pub time: u64,
}

#[derive(Debug, serde::Deserialize)]
pub struct MempoolFees {
    pub base: f64,
}

impl MempoolFees {
    pub fn to_sat(&self) -> u64 {
        (self.base * 100_000_000.0).round() as u64
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct MiningInfo {
    pub blocks: u64,
    pub difficulty: f64,
    #[serde(rename = "networkhashps")]
    pub network_hash_ps: f64,
    #[serde(rename = "pooledtx")]
    pub pooled_tx: u64,
    pub chain: String,
}
