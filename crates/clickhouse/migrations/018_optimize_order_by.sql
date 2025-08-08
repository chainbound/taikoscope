-- Migration 018: prepare l2_reorgs for query-side ordering by block number
-- Adds a dedicated sortable column and backfills it; no ORDER BY mutation.
-- This is safe and idempotent on existing deployments.
--
-- Step 1: add a new column dedicated for sorting
ALTER TABLE ${DB}.l2_reorgs
    ADD COLUMN IF NOT EXISTS l2_block_number_sort UInt64 DEFAULT 0 AFTER l2_block_number;

-- Step 2: backfill the sortable column from the existing data
ALTER TABLE ${DB}.l2_reorgs UPDATE l2_block_number_sort = l2_block_number WHERE 1;
