-- One row per transaction output
CREATE TABLE transaction_outputs (
    id              BIGSERIAL PRIMARY KEY,
    txid            TEXT NOT NULL REFERENCES transactions(txid),
    output_index    INTEGER NOT NULL,           -- vout position within the tx
    value_sats      BIGINT NOT NULL,            -- value in satoshis
    script_pubkey   TEXT NOT NULL,              -- hex-encoded
    script_type     TEXT,                       -- p2pkh, p2wpkh, p2tr, op_return, etc.
    address         TEXT,                       -- NULL if script has no address
    is_spent        BOOLEAN NOT NULL DEFAULT FALSE,

    UNIQUE (txid, output_index)
);

-- Used when marking outputs as spent (looking up by outpoint)
CREATE INDEX idx_outputs_txid_index ON transaction_outputs (txid, output_index);

-- Used for address history and balance queries
CREATE INDEX idx_outputs_address ON transaction_outputs (address);
