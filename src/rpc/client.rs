use bitcoin::Block;
use bitcoin::consensus::deserialize;
use bitcoincore_rpc::{Auth, Client, RpcApi};

use crate::config::Config;
use crate::errors::ExplorerError;

/// Wraps the bitcoincore-rpc client with helper methods
/// that return rust-bitcoin types directly.
pub struct RpcClient {
    inner: Client,
}

impl RpcClient {
    /// Create a new RPC client from config.
    pub fn new(config: &Config) -> Result<Self, ExplorerError> {
        let auth = Auth::UserPass(config.rpc_user.clone(), config.rpc_password.clone());
        let client = Client::new(&config.rpc_url, auth)?;
        Ok(Self { inner: client })
    }

    /// Returns the current best block height.
    pub fn get_block_count(&self) -> Result<u64, ExplorerError> {
        Ok(self.inner.get_block_count()?)
    }

    /// Returns the block hash at a given height.
    pub fn get_block_hash(&self, height: u64) -> Result<bitcoin::BlockHash, ExplorerError> {
        Ok(self.inner.get_block_hash(height)?)
    }

    /// Fetches a raw block at verbosity 0 and decodes it into a rust-bitcoin Block.
    /// Verbosity 0 returns raw hex bytes — faster than verbose JSON.
    pub fn get_block(&self, hash: &bitcoin::BlockHash) -> Result<Block, ExplorerError> {
        let hex_bytes = self.inner.get_block_hex(hash)?;
        let raw = hex::decode(&hex_bytes)?;
        let block: Block = deserialize(&raw)?;
        Ok(block)
    }

    /// Returns general chain info: network, height, best block hash.
    pub fn get_blockchain_info(
        &self,
    ) -> Result<bitcoincore_rpc::json::GetBlockchainInfoResult, ExplorerError> {
        Ok(self.inner.get_blockchain_info()?)
    }

    /// Returns block header info including height, given a block hash.
    pub fn get_block_info(
        &self,
        hash: &bitcoin::BlockHash,
    ) -> Result<bitcoincore_rpc::json::GetBlockHeaderResult, ExplorerError> {
        Ok(self.inner.get_block_header_info(hash)?)
    }

    /// Returns all txids currently in the mempool.
    pub fn get_raw_mempool(&self) -> Result<Vec<bitcoin::Txid>, ExplorerError> {
        Ok(self.inner.get_raw_mempool()?)
    }

    /// Returns details for a specific mempool transaction.
    pub fn get_mempool_entry(
        &self,
        txid: &bitcoin::Txid,
    ) -> Result<bitcoincore_rpc::json::GetMempoolEntryResult, ExplorerError> {
        Ok(self.inner.get_mempool_entry(txid)?)
    }
}
