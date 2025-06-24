-- Migration 007: Add batch_blocks table to enable efficient batch-to-blocks joins
-- This addresses INVALID_JOIN_ON_EXPRESSION errors with ClickHouse 24+ by replacing
-- range joins with equi-joins through a mapping table

-- Create the batch_blocks mapping table
CREATE TABLE IF NOT EXISTS ${DB}.batch_blocks (
    batch_id UInt64,
    l2_block_number UInt64,
    inserted_at DateTime64(3) DEFAULT now64()
) ENGINE = MergeTree()
ORDER BY (batch_id, l2_block_number);

-- Backfill the table with existing batch data
-- Generate all L2 block numbers for each batch based on batch_size and last_l2_block_number
INSERT INTO ${DB}.batch_blocks (batch_id, l2_block_number)
SELECT
    batch_id,
    l2_block_number
FROM (
    SELECT
        batch_id,
        if(
            last_l2_block_number = 0 AND batch_size > 0,
            0,
            last_l2_block_number - batch_size + 1
        ) + number AS l2_block_number
    FROM ${DB}.batches
    ARRAY JOIN range(
        if(last_l2_block_number = 0 AND batch_size > 0, 1, batch_size)
    ) AS number
    WHERE batch_size > 0
      AND l2_block_number <= last_l2_block_number
);