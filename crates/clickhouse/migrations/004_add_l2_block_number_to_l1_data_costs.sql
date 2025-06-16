ALTER TABLE ${DB}.l1_data_costs
ADD COLUMN IF NOT EXISTS l2_block_number UInt64 AFTER l1_block_number;
