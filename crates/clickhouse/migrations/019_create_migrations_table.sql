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
INSERT INTO ${DB}.schema_migrations (version, name, applied_at, checksum) VALUES
    (1, '001_create_tables.sql', now64(), ''),
    (2, '002_create_materialized_views.sql', now64(), ''),
    (3, '003_add_sum_base_fee_column.sql', now64(), ''),
    (4, '004_add_l2_block_number_to_l1_data_costs.sql', now64(), ''),
    (5, '005_create_metrics_views.sql', now64(), ''),
    (6, '006_add_last_l2_block_to_batches.sql', now64(), ''),
    (7, '007_add_batch_blocks_table.sql', now64(), ''),
    (8, '008_add_prove_verify_costs.sql', now64(), ''),
    (9, '009_change_l1_data_costs_to_batch.sql', now64(), ''),
    (10, '010_add_reorg_sequencers.sql', now64(), ''),
    (11, '011_add_tx_hash_to_batches.sql', now64(), ''),
    (12, '012_create_partitioned_head_events.sql', now64(), ''),
    (13, '013_create_partitioned_batches.sql', now64(), ''),
    (14, '014_cleanup_partitioned_head_events.sql', now64(), ''),
    (15, '015_create_orphaned_l2_hashes.sql', now64(), ''),
    (16, '016_add_data_skipping_indices.sql', now64(), ''),
    (17, '017_add_batch_blocks_projection.sql', now64(), ''),
    (18, '018_optimize_order_by.sql', now64(), '');