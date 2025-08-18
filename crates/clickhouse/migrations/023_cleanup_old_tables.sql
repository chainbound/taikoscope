-- Migration 023: Cleanup old tables after ReplacingMergeTree migration
-- This migration cleans up backup tables created during the ReplacingMergeTree transition
-- Safe to run on startup - uses IF EXISTS

-- Clean up old backup tables (safe with IF EXISTS)
DROP TABLE IF EXISTS ${DB}.batches_old;
DROP TABLE IF EXISTS ${DB}.batch_blocks_old;

-- Clean up any leftover temporary tables from migration attempts
DROP TABLE IF EXISTS ${DB}.batches_rmt;
DROP TABLE IF EXISTS ${DB}.batch_blocks_rmt;
DROP TABLE IF EXISTS ${DB}.batches_rmt_backup;
DROP TABLE IF EXISTS ${DB}.batch_blocks_rmt_backup;

-- Mark this migration as applied
INSERT INTO ${DB}.schema_migrations (version, description) 
SELECT '023', 'cleanup_old_tables_after_replacing_merge_tree'
WHERE NOT EXISTS (
    SELECT 1 FROM ${DB}.schema_migrations WHERE version = '023'
);