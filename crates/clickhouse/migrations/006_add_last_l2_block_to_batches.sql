ALTER TABLE ${DB}.batches
ADD COLUMN IF NOT EXISTS last_l2_block_number UInt64 AFTER batch_size;