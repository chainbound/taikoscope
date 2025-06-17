CREATE MATERIALIZED VIEW IF NOT EXISTS ${DB}.daily_l2_metrics_mv
(
    day Date,
    min_ts_state AggregateFunction(min, UInt64),
    max_ts_state AggregateFunction(max, UInt64),
    cnt_state AggregateFunction(count, UInt64),
    tx_sum_state AggregateFunction(sum, UInt64),
    gas_sum_state AggregateFunction(sum, UInt128),
    priority_fee_sum_state AggregateFunction(sum, UInt128),
    base_fee_sum_state AggregateFunction(sum, UInt128)
) ENGINE = AggregatingMergeTree()
PARTITION BY day
ORDER BY day
AS
SELECT
    toDate(h.block_ts) AS day,
    minState(h.block_ts) AS min_ts_state,
    maxState(h.block_ts) AS max_ts_state,
    countState() AS cnt_state,
    sumState(toUInt64(sum_tx)) AS tx_sum_state,
    sumState(sum_gas_used) AS gas_sum_state,
    sumState(sum_priority_fee) AS priority_fee_sum_state,
    sumState(sum_base_fee) AS base_fee_sum_state
FROM ${DB}.l2_head_events h
GROUP BY day;

CREATE MATERIALIZED VIEW IF NOT EXISTS ${DB}.daily_batch_metrics_mv
(
    day Date,
    min_ts_state AggregateFunction(min, UInt64),
    max_ts_state AggregateFunction(max, UInt64),
    cnt_state AggregateFunction(count, UInt64),
    blob_avg_state AggregateFunction(avg, Float64)
) ENGINE = AggregatingMergeTree()
PARTITION BY day
ORDER BY day
AS
SELECT
    toDate(l1.block_ts) AS day,
    minState(toUInt64(l1.block_ts * 1000)) AS min_ts_state,
    maxState(toUInt64(l1.block_ts * 1000)) AS max_ts_state,
    countState() AS cnt_state,
    avgState(toFloat64(b.blob_count)) AS blob_avg_state
FROM ${DB}.batches b
INNER JOIN ${DB}.l1_head_events l1 ON b.l1_block_number = l1.l1_block_number
GROUP BY day;
CREATE MATERIALIZED VIEW IF NOT EXISTS ${DB}.hourly_l2_metrics_mv
(
    hour DateTime64(3),
    min_ts_state AggregateFunction(min, UInt64),
    max_ts_state AggregateFunction(max, UInt64),
    cnt_state AggregateFunction(count, UInt64),
    tx_sum_state AggregateFunction(sum, UInt64),
    gas_sum_state AggregateFunction(sum, UInt128),
    priority_fee_sum_state AggregateFunction(sum, UInt128),
    base_fee_sum_state AggregateFunction(sum, UInt128)
) ENGINE = AggregatingMergeTree()
PARTITION BY toDate(hour)
ORDER BY hour
AS
SELECT
    toStartOfHour(fromUnixTimestamp64Milli(h.block_ts * 1000)) AS hour,
    minState(h.block_ts) AS min_ts_state,
    maxState(h.block_ts) AS max_ts_state,
    countState() AS cnt_state,
    sumState(toUInt64(sum_tx)) AS tx_sum_state,
    sumState(sum_gas_used) AS gas_sum_state,
    sumState(sum_priority_fee) AS priority_fee_sum_state,
    sumState(sum_base_fee) AS base_fee_sum_state
FROM ${DB}.l2_head_events h
GROUP BY hour;

CREATE MATERIALIZED VIEW IF NOT EXISTS ${DB}.hourly_batch_metrics_mv
(
    hour DateTime64(3),
    min_ts_state AggregateFunction(min, UInt64),
    max_ts_state AggregateFunction(max, UInt64),
    cnt_state AggregateFunction(count, UInt64),
    blob_avg_state AggregateFunction(avg, Float64)
) ENGINE = AggregatingMergeTree()
PARTITION BY toDate(hour)
ORDER BY hour
AS
SELECT
    toStartOfHour(fromUnixTimestamp64Milli(l1.block_ts * 1000)) AS hour,
    minState(toUInt64(l1.block_ts * 1000)) AS min_ts_state,
    maxState(toUInt64(l1.block_ts * 1000)) AS max_ts_state,
    countState() AS cnt_state,
    avgState(toFloat64(b.blob_count)) AS blob_avg_state
FROM ${DB}.batches b
INNER JOIN ${DB}.l1_head_events l1 ON b.l1_block_number = l1.l1_block_number
GROUP BY hour;
