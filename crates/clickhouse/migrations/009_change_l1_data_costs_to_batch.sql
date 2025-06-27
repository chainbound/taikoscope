ALTER TABLE ${DB}.l1_data_costs
ADD COLUMN IF NOT EXISTS batch_id UInt64 AFTER l1_block_number;

ALTER TABLE ${DB}.l1_data_costs
DROP COLUMN IF EXISTS l2_block_number;
