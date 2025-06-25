-- Migration 008: Add prove_costs and verify_costs tables

CREATE TABLE IF NOT EXISTS ${DB}.prove_costs (
    l1_block_number UInt64,
    batch_id UInt64,
    cost UInt128,
    inserted_at DateTime64(3) DEFAULT now64()
) ENGINE = MergeTree()
ORDER BY (l1_block_number, batch_id);

CREATE TABLE IF NOT EXISTS ${DB}.verify_costs (
    l1_block_number UInt64,
    batch_id UInt64,
    cost UInt128,
    inserted_at DateTime64(3) DEFAULT now64()
) ENGINE = MergeTree()
ORDER BY (l1_block_number, batch_id);
