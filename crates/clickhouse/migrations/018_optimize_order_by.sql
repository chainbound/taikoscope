-- Migration 018: Optimize ORDER BY keys to match common filters
-- Rationale:
-- - proved_batches / verified_batches are frequently filtered/joined by batch_id,
--   so lead with batch_id in the sorting key for better index granularity.
-- - l2_reorgs is primarily time-filtered but often displayed around a block;
--   keep inserted_at first and add l2_block_number as a secondary key.

ALTER TABLE ${DB}.proved_batches
    MODIFY ORDER BY (batch_id, l1_block_number);

ALTER TABLE ${DB}.verified_batches
    MODIFY ORDER BY (batch_id, l1_block_number);

ALTER TABLE ${DB}.l2_reorgs
    MODIFY ORDER BY (inserted_at, l2_block_number);
