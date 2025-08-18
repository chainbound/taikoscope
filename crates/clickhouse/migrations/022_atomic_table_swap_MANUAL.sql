-- Migration 022: Atomic Table Swap (MANUAL EXECUTION REQUIRED)
-- This migration performs the final atomic swap to activate ReplacingMergeTree tables
-- 
-- ⚠️  IMPORTANT: This migration should NOT run on startup automatically
-- ⚠️  Execute manually during a maintenance window
-- ⚠️  Application MUST be stopped during this operation
--
-- Prerequisites:
-- 1. Migration 021 completed successfully (data migrated)
-- 2. Application is stopped (no active writes)
-- 3. Data integrity verified
--
-- Execution time: < 1 second (atomic operations)

-- Final verification before swap
SELECT 'Pre-swap verification' as status;

-- Verify data counts match expectations
SELECT 
    (SELECT count(DISTINCT batch_id) FROM ${DB}.batches) as original_batches,
    (SELECT count(*) FROM ${DB}.batches_rmt) as new_batches;

SELECT 
    (SELECT count(DISTINCT batch_id, l2_block_number) FROM ${DB}.batch_blocks) as original_batch_blocks,
    (SELECT count(*) FROM ${DB}.batch_blocks_rmt) as new_batch_blocks;

-- Verify _rmt tables are using ReplacingMergeTree engine
SELECT name, engine 
FROM system.tables 
WHERE database = '${DB}' 
  AND name IN ('batches_rmt', 'batch_blocks_rmt')
  AND engine LIKE '%ReplacingMergeTree%';

-- If verification above looks good, proceed with atomic swap

-- Step 1: Clean up any leftover tables from previous attempts
DROP TABLE IF EXISTS ${DB}.batches_old;
DROP TABLE IF EXISTS ${DB}.batch_blocks_old;

-- Step 2: Atomic table swap
-- These operations are atomic and instantaneous
RENAME TABLE ${DB}.batches TO ${DB}.batches_old;
RENAME TABLE ${DB}.batch_blocks TO ${DB}.batch_blocks_old;

-- Step 3: Move ReplacingMergeTree tables into place
RENAME TABLE ${DB}.batches_rmt TO ${DB}.batches;
RENAME TABLE ${DB}.batch_blocks_rmt TO ${DB}.batch_blocks;

-- Step 4: Verification
SELECT 'Post-swap verification' as status;

-- Verify new tables are active and using correct engine
SELECT name, engine 
FROM system.tables 
WHERE database = '${DB}' 
  AND name IN ('batches', 'batch_blocks')
  AND engine LIKE '%ReplacingMergeTree%';

-- Verify data is accessible
SELECT count(*) as active_batches FROM ${DB}.batches LIMIT 1;
SELECT count(*) as active_batch_blocks FROM ${DB}.batch_blocks LIMIT 1;

-- Mark this migration as applied
INSERT INTO ${DB}.schema_migrations (version, description) 
SELECT '022', 'atomic_table_swap_to_replacing_merge_tree'
WHERE NOT EXISTS (
    SELECT 1 FROM ${DB}.schema_migrations WHERE version = '022'
);

SELECT 'Table swap completed successfully! Application can be restarted.' as final_status;
SELECT 'Old tables preserved as batches_old and batch_blocks_old for rollback' as note;

-- ROLLBACK Instructions (if needed):
-- RENAME TABLE ${DB}.batches TO ${DB}.batches_rmt_backup;
-- RENAME TABLE ${DB}.batch_blocks TO ${DB}.batch_blocks_rmt_backup;
-- RENAME TABLE ${DB}.batches_old TO ${DB}.batches;
-- RENAME TABLE ${DB}.batch_blocks_old TO ${DB}.batch_blocks;