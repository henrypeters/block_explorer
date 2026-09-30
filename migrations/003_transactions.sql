-- One row per transaction
CREATE TABLE transactions (
    txid            TEXT PRIMARY KEY,
    block_height    INTEGER NOT NULL REFERENCES blocks(height),
    block_hash      TEXT NOT NULL,
    tx_index        INTEGER NOT NULL,           -- position of tx within the block
    version         INTEGER NOT NULL,
    locktime        BIGINT NOT NULL,
    is_coinbase     BOOLEAN NOT NULL,
    size            INTEGER NOT NULL,           -- bytes
    indexed_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_transactions_block_height ON transactions (block_height);
