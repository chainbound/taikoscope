-- Migration 013: Create partitioned shadow table for batches

CREATE TABLE IF NOT EXISTS ${DB}.batches_p (
    l1_block_number     UInt64,
    l1_tx_hash          FixedString(32),
    batch_id            UInt64,
    batch_size          UInt16,
    last_l2_block_number UInt64,
    proposer_addr       FixedString(20),
    blob_count          UInt8,
    blob_total_bytes    UInt32,
    inserted_at         DateTime64(3) DEFAULT now64()
) ENGINE = MergeTree()
PARTITION BY toDate(inserted_at)
ORDER BY (toStartOfDay(inserted_at), l1_block_number, batch_id);

-- Backfill last 30 days of data
INSERT INTO ${DB}.batches_p
SELECT *
FROM ${DB}.batches
WHERE inserted_at >= now64() - INTERVAL 30 DAY;

-- Ensure no leftover shadow table
DROP TABLE IF EXISTS ${DB}.batches_old;

-- Swap batches table (single-table renames only)
RENAME TABLE ${DB}.batches    TO ${DB}.batches_old;
RENAME TABLE ${DB}.batches_p  TO ${DB}.batches;