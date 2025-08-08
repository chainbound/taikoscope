-- Rationale:
-- - Keep existing keys for proved_batches / verified_batches because their
--   primary key is (l1_block_number, batch_id). ClickHouse requires the
--   primary key to be a prefix of ORDER BY, so reordering to (batch_id, l1_block_number)
--   would violate that constraint. We leave these tables unchanged.
-- - l2_reorgs is primarily time-filtered but often displayed around a block;
--   keep inserted_at first and add a secondary key for block navigation.
--   ClickHouse 25.x disallows adding existing columns directly into ORDER BY.
--   To comply, we add a new sortable column, backfill it, then update ORDER BY.
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
--   -- Revert l2_reorgs ORDER BY key
--   -- ALTER TABLE ${DB}.l2_reorgs MODIFY ORDER BY (inserted_at) SETTINGS mutations_sync = 0, replication_alter_partitions_sync = 2;

-- Step 1: add a new column dedicated for sorting (cannot reference existing columns in ORDER BY modification)
ALTER TABLE ${DB}.l2_reorgs
    ADD COLUMN IF NOT EXISTS l2_block_number_sort UInt64 DEFAULT 0 AFTER l2_block_number;

-- Step 2: backfill the sortable column from the existing data
ALTER TABLE ${DB}.l2_reorgs UPDATE l2_block_number_sort = l2_block_number WHERE 1;

-- Note: ClickHouse 25.x rejects modifying ORDER BY using a column once it exists,
-- and this cannot be done safely online without a deep table rewrite.
-- We keep the new sortable column for query use and skip modifying ORDER BY.
