-- Migration 019: Add schema migrations tracking table
-- This enables idempotent migrations and prevents duplicate execution

CREATE TABLE IF NOT EXISTS ${DB}.schema_migrations (
    version String,
    description String,
    applied_at DateTime DEFAULT now()
) ENGINE = MergeTree() 
ORDER BY version;

-- Insert records for all existing migrations that would have been applied
-- This assumes migrations 001-018 have already been executed
INSERT INTO ${DB}.schema_migrations (version, description, applied_at) 
SELECT version, description, now() as applied_at
FROM (
    SELECT '001' as version, 'create_tables' as description
    UNION ALL SELECT '002', 'create_materialized_views'
    UNION ALL SELECT '003', 'add_sum_base_fee_column'
    UNION ALL SELECT '004', 'add_l2_block_number_to_l1_data_costs'
    UNION ALL SELECT '005', 'create_metrics_views'
    UNION ALL SELECT '006', 'add_last_l2_block_to_batches'
    UNION ALL SELECT '007', 'add_batch_blocks_table'
    UNION ALL SELECT '008', 'add_prove_verify_costs'
    UNION ALL SELECT '009', 'change_l1_data_costs_to_batch'
    UNION ALL SELECT '010', 'add_reorg_sequencers'
    UNION ALL SELECT '011', 'add_tx_hash_to_batches'
    UNION ALL SELECT '012', 'create_partitioned_head_events'
    UNION ALL SELECT '013', 'create_partitioned_batches'
    UNION ALL SELECT '014', 'cleanup_partitioned_head_events'
    UNION ALL SELECT '015', 'create_orphaned_l2_hashes'
    UNION ALL SELECT '016', 'add_data_skipping_indices'
    UNION ALL SELECT '017', 'add_batch_blocks_projection'
    UNION ALL SELECT '018', 'optimize_order_by'
    UNION ALL SELECT '019', 'add_schema_migrations_tracking'
) AS existing_migrations
WHERE NOT EXISTS (
    SELECT 1 FROM ${DB}.schema_migrations WHERE version = existing_migrations.version
);