-- Create schema_migrations table to track applied migrations
CREATE TABLE IF NOT EXISTS ${DB}.schema_migrations (
    version UInt32,
    name String,
    applied_at DateTime64(3) DEFAULT now64(),
    checksum String
) ENGINE = MergeTree()
ORDER BY (version);

-- Insert record for existing migrations (assuming they were already applied)
-- This is a one-time bootstrap for systems that already have migrations applied
-- Only run if base tables exist (not a fresh install) and migrations don't already exist
INSERT INTO ${DB}.schema_migrations (version, name, applied_at, checksum)
SELECT v, n, now64(), ''
FROM (
    SELECT 1 v, '001_create_tables.sql' n UNION ALL
    SELECT 2, '002_create_materialized_views.sql' UNION ALL
    SELECT 3, '003_add_sum_base_fee_column.sql' UNION ALL
    SELECT 4, '004_add_l2_block_number_to_l1_data_costs.sql' UNION ALL
    SELECT 5, '005_create_metrics_views.sql' UNION ALL
    SELECT 6, '006_add_last_l2_block_to_batches.sql' UNION ALL
    SELECT 7, '007_add_batch_blocks_table.sql' UNION ALL
    SELECT 8, '008_add_prove_verify_costs.sql' UNION ALL
    SELECT 9, '009_change_l1_data_costs_to_batch.sql' UNION ALL
    SELECT 10, '010_add_reorg_sequencers.sql' UNION ALL
    SELECT 11, '011_add_tx_hash_to_batches.sql' UNION ALL
    SELECT 12, '012_create_partitioned_head_events.sql' UNION ALL
    SELECT 13, '013_create_partitioned_batches.sql' UNION ALL
    SELECT 14, '014_cleanup_partitioned_head_events.sql' UNION ALL
    SELECT 15, '015_create_orphaned_l2_hashes.sql' UNION ALL
    SELECT 16, '016_add_data_skipping_indices.sql' UNION ALL
    SELECT 17, '017_add_batch_blocks_projection.sql' UNION ALL
    SELECT 18, '018_optimize_order_by.sql'
) s
WHERE
    -- only if base tables exist (not a fresh install)
    EXISTS (SELECT 1 FROM system.tables WHERE database='${DB}' AND name='batches')
    AND NOT EXISTS (SELECT 1 FROM ${DB}.schema_migrations WHERE version = s.v);