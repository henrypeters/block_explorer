-- Speed up the block list query used by the TUI
CREATE INDEX IF NOT EXISTS idx_blocks_height_desc ON blocks (height DESC);
