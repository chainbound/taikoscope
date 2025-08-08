-- Migration 018: Optimize ORDER BY keys to match common filters
-- Rationale:
-- - proved_batches / verified_batches are frequently filtered/joined by batch_id,
--   so lead with batch_id in the sorting key for better index granularity.
-- - l2_reorgs is primarily time-filtered but often displayed around a block;
--   keep inserted_at first and add l2_block_number as a secondary key.
--
-- Operational notes:
-- - This issues asynchronous mutations to avoid long blocking rewrites on large tables.
-- - Monitor progress with:
--   SELECT database, table, mutation_id, command, create_time, is_done,
--          latest_failed_part, latest_fail_reason, parts_to_do
--   FROM system.mutations
--   WHERE database = '${DB}' AND table IN ('proved_batches','verified_batches','l2_reorgs')
--   ORDER BY create_time DESC;
--
-- Rollback plan (if needed):
--   -- Revert to original ORDER BY keys
--   -- ALTER TABLE ${DB}.proved_batches   MODIFY ORDER BY (l1_block_number, batch_id) SETTINGS mutations_sync = 0, replication_alter_partitions_sync = 2;
--   -- ALTER TABLE ${DB}.verified_batches MODIFY ORDER BY (l1_block_number, batch_id) SETTINGS mutations_sync = 0, replication_alter_partitions_sync = 2;
--   -- ALTER TABLE ${DB}.l2_reorgs        MODIFY ORDER BY (inserted_at)                SETTINGS mutations_sync = 0, replication_alter_partitions_sync = 2;

ALTER TABLE ${DB}.proved_batches
    MODIFY ORDER BY (batch_id, l1_block_number)
    SETTINGS mutations_sync = 0, replication_alter_partitions_sync = 2;

ALTER TABLE ${DB}.verified_batches
    MODIFY ORDER BY (batch_id, l1_block_number)
    SETTINGS mutations_sync = 0, replication_alter_partitions_sync = 2;

ALTER TABLE ${DB}.l2_reorgs
    MODIFY ORDER BY (inserted_at, l2_block_number)
    SETTINGS mutations_sync = 0, replication_alter_partitions_sync = 2;
