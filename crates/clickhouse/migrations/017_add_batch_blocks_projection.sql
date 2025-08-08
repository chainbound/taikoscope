-- Migration 017: Add projection on batch_blocks for efficient block->batch lookups
-- This complements the ORDER BY (batch_id, l2_block_number) by providing an
-- alternate sorted projection ordered by (l2_block_number, batch_id).

ALTER TABLE ${DB}.batch_blocks
    ADD PROJECTION IF NOT EXISTS by_l2_block
    (
        SELECT batch_id, l2_block_number, inserted_at
        ORDER BY (l2_block_number, batch_id)
    );

-- Materialize the projection for existing parts
ALTER TABLE ${DB}.batch_blocks MATERIALIZE PROJECTION by_l2_block;
