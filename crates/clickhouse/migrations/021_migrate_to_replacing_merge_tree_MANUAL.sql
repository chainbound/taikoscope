-- Migration 021: Data Migration to ReplacingMergeTree (MANUAL EXECUTION REQUIRED)
-- This migration copies and deduplicates data from existing tables to ReplacingMergeTree tables
-- 
-- ⚠️  IMPORTANT: This migration should NOT run on startup automatically
-- ⚠️  Execute manually during a maintenance window
-- 
-- Prerequisites:
-- 1. Migration 020 must be completed (creates _rmt tables)
-- 2. Application should be stopped or in read-only mode
-- 3. Backup existing data before running
--
-- Execution time: Depends on data size (minutes to hours for large datasets)

-- Check prerequisites
-- Verify that _rmt tables exist
SELECT 'Checking prerequisites...' as status;

-- Verify batches_rmt table exists
SELECT 'batches_rmt exists' as check_result
FROM system.tables 
WHERE database = '${DB}' AND name = 'batches_rmt' AND engine LIKE '%ReplacingMergeTree%'
LIMIT 1;

-- Verify batch_blocks_rmt table exists  
SELECT 'batch_blocks_rmt exists' as check_result
FROM system.tables 
WHERE database = '${DB}' AND name = 'batch_blocks_rmt' AND engine LIKE '%ReplacingMergeTree%'
LIMIT 1;

-- Check if data migration already completed
SELECT 'Migration status check' as status;
SELECT count(*) as batches_rmt_count FROM ${DB}.batches_rmt;
SELECT count(*) as batch_blocks_rmt_count FROM ${DB}.batch_blocks_rmt;

-- If counts above are > 0, migration may have already been run
-- Proceed with caution or skip data copying steps

-- Step 1: Copy deduplicated data from batches
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
WHERE batch_id NOT IN (SELECT DISTINCT batch_id FROM ${DB}.batches_rmt)
GROUP BY batch_id;

-- Step 2: Copy deduplicated data from batch_blocks
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
WHERE (batch_id, l2_block_number) NOT IN (
    SELECT batch_id, l2_block_number FROM ${DB}.batch_blocks_rmt
)
GROUP BY batch_id, l2_block_number;

-- Step 3: Materialize the projection (may take time for large datasets)
ALTER TABLE ${DB}.batch_blocks_rmt MATERIALIZE PROJECTION by_l2_block;

-- Step 4: Verify data integrity
SELECT 'Data integrity verification' as status;

-- Count comparison
SELECT 
    (SELECT count(DISTINCT batch_id) FROM ${DB}.batches) as original_batches_count,
    (SELECT count(*) FROM ${DB}.batches_rmt) as new_batches_count;

SELECT 
    (SELECT count(DISTINCT batch_id, l2_block_number) FROM ${DB}.batch_blocks) as original_batch_blocks_count,
    (SELECT count(*) FROM ${DB}.batch_blocks_rmt) as new_batch_blocks_count;

-- Sample verification - check that latest data was preserved
SELECT 'Sample data verification for batches' as status;
SELECT 
    b1.batch_id,
    b1.l1_block_number as original_l1_block,
    b2.l1_block_number as new_l1_block,
    b1.inserted_at as original_time,
    b2.inserted_at as new_time
FROM ${DB}.batches b1
JOIN ${DB}.batches_rmt b2 ON b1.batch_id = b2.batch_id
WHERE b1.inserted_at = (
    SELECT max(inserted_at) 
    FROM ${DB}.batches 
    WHERE batch_id = b1.batch_id
)
LIMIT 10;

-- Ready for table swap? Check this query returns expected results
SELECT 'Ready for table swap verification' as status;
SELECT count(*) as total_new_batches FROM ${DB}.batches_rmt;
SELECT count(*) as total_new_batch_blocks FROM ${DB}.batch_blocks_rmt;

-- Mark this migration as applied  
INSERT INTO ${DB}.schema_migrations (version, description) 
SELECT '021', 'migrate_to_replacing_merge_tree_data'
WHERE NOT EXISTS (
    SELECT 1 FROM ${DB}.schema_migrations WHERE version = '021'
);

SELECT 'Data migration completed. Ready for table swap (migration 022)' as final_status;