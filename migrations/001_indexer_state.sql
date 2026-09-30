CREATE TABLE indexer_state (
    id INTEGER PRIMARY KEY DEFAULT 1,           
    last_indexed_height INTEGER NOT NULL DEFAULT -1,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT single_row CHECK (id = 1)
);

INSERT INTO indexer_state (id, last_indexed_height) VALUES (1, -1);
