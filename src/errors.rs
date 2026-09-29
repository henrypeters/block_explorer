use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExplorerError {
    #[error("RPC error: {0}")]
    Rpc(#[from] bitcoincore_rpc::Error),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Bitcoin decode error: {0}")]
    BitcoinDecode(#[from] bitcoin::consensus::encode::Error),

    #[error("Hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),

    #[error("{0}")]
    Other(String),
}
