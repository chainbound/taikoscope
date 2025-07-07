-- Migration 012: Create partitioned shadow tables for l2_head_events and l1_head_events

CREATE TABLE IF NOT EXISTS ${DB}.l2_head_events_p (
    l2_block_number UInt64,
    block_hash      FixedString(32),
    block_ts        UInt64,
    sum_gas_used    UInt128,
    sum_tx          UInt32,
    sum_priority_fee UInt128,
    sum_base_fee    UInt128,
    sequencer       FixedString(20),
    inserted_at     DateTime64(3) DEFAULT now64()
) ENGINE = MergeTree()
PARTITION BY toDate(fromUnixTimestamp(block_ts))
ORDER BY (block_ts, l2_block_number);

CREATE TABLE IF NOT EXISTS ${DB}.l1_head_events_p (
    l1_block_number UInt64,
    block_hash      FixedString(32),
    slot            UInt64,
    block_ts        UInt64,
    inserted_at     DateTime64(3) DEFAULT now64()
) ENGINE = MergeTree()
PARTITION BY toDate(fromUnixTimestamp(block_ts))
ORDER BY (block_ts, l1_block_number);

-- Backfill last 30 days of data
INSERT INTO ${DB}.l2_head_events_p
SELECT *
FROM ${DB}.l2_head_events
WHERE block_ts >= (toUInt64(now()) - 30*24*3600);

INSERT INTO ${DB}.l1_head_events_p
SELECT *
FROM ${DB}.l1_head_events
WHERE block_ts >= (toUInt64(now()) - 30*24*3600);

-- Ensure no leftover old tables before swapping
DROP TABLE IF EXISTS ${DB}.l2_head_events_old;
DROP TABLE IF EXISTS ${DB}.l1_head_events_old;

-- Atomic swap: rename old tables and move partitioned tables into place
RENAME TABLE ${DB}.l2_head_events TO       ${DB}.l2_head_events_old;
RENAME TABLE ${DB}.l2_head_events_p TO     ${DB}.l2_head_events;

RENAME TABLE ${DB}.l1_head_events TO       ${DB}.l1_head_events_old;
RENAME TABLE ${DB}.l1_head_events_p TO     ${DB}.l1_head_events;