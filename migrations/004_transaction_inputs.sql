-- One row per transaction input
CREATE TABLE transaction_inputs (
    id              BIGSERIAL PRIMARY KEY,
    txid            TEXT NOT NULL REFERENCES transactions(txid),
    input_index     INTEGER NOT NULL,           -- position of input within the tx
    prev_txid       TEXT,                       -- NULL for coinbase
    prev_vout       INTEGER,                    -- NULL for coinbase
    script_sig      TEXT NOT NULL,              -- hex-encoded
    sequence        BIGINT NOT NULL,
    witness         TEXT[],                     -- array of hex-encoded witness items

    UNIQUE (txid, input_index)
);

-- This index is critical: used to find the spending tx for a given output
CREATE INDEX idx_inputs_prev_txid_vout ON transaction_inputs (prev_txid, prev_vout);
