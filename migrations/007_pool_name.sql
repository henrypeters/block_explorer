-- Add pool_name column to blocks for coinbase analysis
ALTER TABLE blocks ADD COLUMN IF NOT EXISTS pool_name TEXT;

-- Index for pool queries
CREATE INDEX IF NOT EXISTS idx_blocks_pool_name ON blocks (pool_name);
