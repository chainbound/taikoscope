-- Migration 019: Create orphaned_l1_hashes table and index

CREATE TABLE IF NOT EXISTS ${DB}.orphaned_l1_hashes (
    block_hash FixedString(32),
    l1_block_number UInt64,
    inserted_at DateTime64(3) DEFAULT now64()
) ENGINE = MergeTree()
ORDER BY (l1_block_number, block_hash);

-- orphaned_l1_hashes: lookups by block_hash
ALTER TABLE ${DB}.orphaned_l1_hashes
    ADD INDEX IF NOT EXISTS idx_orphaned_l1_block_hash_bf block_hash TYPE bloom_filter(0.01) GRANULARITY 1;
ALTER TABLE ${DB}.orphaned_l1_hashes MATERIALIZE INDEX idx_orphaned_l1_block_hash_bf;
