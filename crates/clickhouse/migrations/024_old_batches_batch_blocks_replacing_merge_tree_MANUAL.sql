-- Migration 024: OLD VERSION - Convert batches and batch_blocks to ReplacingMergeTree for automatic deduplication
-- This migration creates new ReplacingMergeTree tables to handle duplicates from gap detection and backfill

-- 1. Create new batches table with ReplacingMergeTree
-- Using batch_id as the primary ORDER BY key for optimal deduplication
-- Adding monthly partitioning for better data management
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

-- 2. Create new batch_blocks table with ReplacingMergeTree
-- Keeping existing ORDER BY since it's optimal for the composite unique key
-- Adding monthly partitioning for consistency
CREATE TABLE IF NOT EXISTS ${DB}.batch_blocks_rmt (
    batch_id            UInt64,
    l2_block_number     UInt64,
    inserted_at         DateTime64(3) DEFAULT now64()
) ENGINE = ReplacingMergeTree(inserted_at)
PARTITION BY toYYYYMM(inserted_at)
ORDER BY (batch_id, l2_block_number);

-- 3. Copy deduplicated data from batches
-- Use argMax() to ensure all columns are taken from the record with the latest inserted_at
INSERT INTO ${DB}.batches_rmt (
    l1_block_number,
    l1_tx_hash,
    batch_id,
    batch_size,
    last_l2_block_number,
    proposer_addr,
    blob_count,
    blob_total_bytes,
    inserted_at
)
SELECT 
    argMax(l1_block_number, inserted_at) as l1_block_number,
    argMax(l1_tx_hash, inserted_at) as l1_tx_hash,
    batch_id,
    argMax(batch_size, inserted_at) as batch_size,
    argMax(last_l2_block_number, inserted_at) as last_l2_block_number,
    argMax(proposer_addr, inserted_at) as proposer_addr,
    argMax(blob_count, inserted_at) as blob_count,
    argMax(blob_total_bytes, inserted_at) as blob_total_bytes,
    max(inserted_at) as inserted_at
FROM ${DB}.batches
GROUP BY batch_id;

-- 4. Copy deduplicated data from batch_blocks
-- Use GROUP BY (batch_id, l2_block_number) to deduplicate
INSERT INTO ${DB}.batch_blocks_rmt (
    batch_id,
    l2_block_number,
    inserted_at
)
SELECT 
    batch_id,
    l2_block_number,
    max(inserted_at) as inserted_at
FROM ${DB}.batch_blocks
GROUP BY batch_id, l2_block_number;

-- 5. Add projection for batch_blocks_rmt (same as original)
-- This maintains query performance for l2_block_number lookups
ALTER TABLE ${DB}.batch_blocks_rmt
    ADD PROJECTION IF NOT EXISTS by_l2_block
    (
        SELECT batch_id, l2_block_number, inserted_at
        ORDER BY (l2_block_number, batch_id)
    );

-- 6. Materialize the projection for existing data
ALTER TABLE ${DB}.batch_blocks_rmt MATERIALIZE PROJECTION by_l2_block;

-- 7. Ensure no leftover shadow tables before swapping
DROP TABLE IF EXISTS ${DB}.batches_old;
DROP TABLE IF EXISTS ${DB}.batch_blocks_old;

-- 8. Atomic table swap
-- Rename existing tables to _old suffix
RENAME TABLE ${DB}.batches TO ${DB}.batches_old;
RENAME TABLE ${DB}.batch_blocks TO ${DB}.batch_blocks_old;

-- Move new ReplacingMergeTree tables into place
RENAME TABLE ${DB}.batches_rmt TO ${DB}.batches;
RENAME TABLE ${DB}.batch_blocks_rmt TO ${DB}.batch_blocks;

-- Note: Old tables (batches_old, batch_blocks_old) are preserved for rollback
-- They can be dropped manually after validation with:
-- DROP TABLE ${DB}.batches_old;
-- DROP TABLE ${DB}.batch_blocks_old;