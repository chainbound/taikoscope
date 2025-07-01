ALTER TABLE ${DB}.batches
ADD COLUMN IF NOT EXISTS l1_tx_hash FixedString(32) AFTER l1_block_number;
