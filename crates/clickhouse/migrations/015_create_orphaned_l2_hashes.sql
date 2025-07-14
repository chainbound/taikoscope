-- Migration 015: Create orphaned_l2_hashes table to track orphaned block hashes

CREATE TABLE IF NOT EXISTS ${DB}.orphaned_l2_hashes (
    block_hash FixedString(32),
    l2_block_number UInt64,
    inserted_at DateTime64(3) DEFAULT now64()
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(inserted_at)
ORDER BY (block_hash, l2_block_number);