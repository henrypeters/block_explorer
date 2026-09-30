use std::env;

/// Holds all configuration needed to run the block explorer.
/// Values are read from environment variables (or a .env file).
#[derive(Debug, Clone)]
pub struct Config {
    /// Bitcoin Core RPC URL e.g. http://127.0.0.1:18443
    pub rpc_url: String,

    /// Bitcoin Core RPC username
    pub rpc_user: String,

    /// Bitcoin Core RPC password
    pub rpc_password: String,

    /// PostgreSQL connection string
    pub database_url: String,

    /// Bitcoin network: "regtest", "testnet", or "mainnet"
    pub network: String,

    /// ZMQ address for new block notifications e.g. tcp://127.0.0.1:28334
    pub zmq_block_url: String,
}

impl Config {
    /// Load configuration from environment variables.
    /// Panics early with a clear message if a required variable is missing.
    pub fn from_env() -> Self {
        Self {
            rpc_url: require_env("RPC_URL"),
            rpc_user: require_env("RPC_USER"),
            rpc_password: require_env("RPC_PASSWORD"),
            database_url: require_env("DATABASE_URL"),
            network: env::var("BITCOIN_NETWORK").unwrap_or_else(|_| "regtest".to_string()),
            zmq_block_url: env::var("ZMQ_BLOCK_URL")
                .unwrap_or_else(|_| "tcp://127.0.0.1:28334".to_string()),
        }
    }
}

/// Reads an environment variable and panics with a clear message if it is missing.
fn require_env(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("Missing required environment variable: {key}"))
}
