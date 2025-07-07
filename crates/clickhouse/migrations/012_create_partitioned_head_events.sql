-- Migration 012: Create partitioned shadow tables for l2_head_events and l1_head_events

CREATE TABLE IF NOT EXISTS ${DB}.l2_head_events_p (
    l2_block_number UInt64,
    block_hash FixedString(32),
    block_ts UInt64,
    sum_gas_used UInt128,
    sum_tx UInt32,
    sum_priority_fee UInt128,
    sum_base_fee UInt128,
    sequencer FixedString(20),
    inserted_at DateTime64(3) DEFAULT now64()
) ENGINE = MergeTree()
PARTITION BY toDate(fromUnixTimestamp(block_ts))
ORDER BY (block_ts, l2_block_number);

CREATE TABLE IF NOT EXISTS ${DB}.l1_head_events_p (
    l1_block_number UInt64,
    block_hash FixedString(32),
    slot UInt64,
    block_ts UInt64,
    inserted_at DateTime64(3) DEFAULT now64()
) ENGINE = MergeTree()
PARTITION BY toDate(fromUnixTimestamp(block_ts))
ORDER BY (block_ts, l1_block_number);