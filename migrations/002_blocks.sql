-- One row per block indexed from Bitcoin Core
CREATE TABLE blocks (
    height          INTEGER PRIMARY KEY,
    hash            TEXT NOT NULL UNIQUE,
    version         INTEGER NOT NULL,
    prev_hash       TEXT NOT NULL,
    merkle_root     TEXT NOT NULL,
    timestamp       BIGINT NOT NULL,            -- unix timestamp
    bits            BIGINT NOT NULL,
    nonce           BIGINT NOT NULL,
    size            INTEGER NOT NULL,           -- bytes
    tx_count        INTEGER NOT NULL,
    indexed_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_blocks_hash ON blocks (hash);
