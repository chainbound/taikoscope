-- Migration 020: Create ReplacingMergeTree shadow tables (SAFE - Idempotent)
-- This is part 1 of the ReplacingMergeTree migration
-- Safe to run on startup multiple times

-- Only run if not already applied
-- Check if this migration has been applied
-- If schema_migrations table doesn't exist, this will safely pass through

-- Create new batches table with ReplacingMergeTree
CREATE TABLE IF NOT EXISTS ${DB}.batches_rmt (
    l1_block_number     UInt64,
    l1_tx_hash          FixedString(32),
    batch_id            UInt64,
    batch_size          UInt16,
    last_l2_block_number UInt64,
    proposer_addr       FixedString(20),
    blob_count          UInt8,
    blob_total_bytes    UInt32,
    inserted_at         DateTime64(3) DEFAULT now64()
) ENGINE = ReplacingMergeTree(inserted_at)
PARTITION BY toYYYYMM(inserted_at)
ORDER BY (batch_id);

-- Add data skipping indexes for optimal query performance across all patterns
-- Index for time-based queries (monitoring: get_unproved_batches_older_than, get_unverified_batches_older_than)
ALTER TABLE ${DB}.batches_rmt ADD INDEX IF NOT EXISTS idx_inserted_at inserted_at TYPE minmax GRANULARITY 1;

-- Index for L1 block-based queries (get_last_batch_time, get_batch_posting_*, etc.)
ALTER TABLE ${DB}.batches_rmt ADD INDEX IF NOT EXISTS idx_l1_block_number l1_block_number TYPE minmax GRANULARITY 1;

-- Create new batch_blocks table with ReplacingMergeTree
CREATE TABLE IF NOT EXISTS ${DB}.batch_blocks_rmt (
    batch_id            UInt64,
    l2_block_number     UInt64,
    inserted_at         DateTime64(3) DEFAULT now64()
) ENGINE = ReplacingMergeTree(inserted_at)
PARTITION BY toYYYYMM(inserted_at)
ORDER BY (batch_id, l2_block_number);

-- Add projection for batch_blocks_rmt (idempotent)
ALTER TABLE ${DB}.batch_blocks_rmt
    ADD PROJECTION IF NOT EXISTS by_l2_block
    (
        SELECT batch_id, l2_block_number, inserted_at
        ORDER BY (l2_block_number, batch_id)
    );

-- Mark this migration as applied
INSERT INTO ${DB}.schema_migrations (version, description) 
SELECT '020', 'create_replacing_merge_tree_tables'
WHERE NOT EXISTS (
    SELECT 1 FROM ${DB}.schema_migrations WHERE version = '020'
);