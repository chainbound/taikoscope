//! Data aggregation utilities

use api_types::BlockTransactionsItem;

use api_types::{AvgBatchBlobCountRow, BatchFeeComponentRow};
use clickhouse_lib::{
    BatchBlobCountRow, BatchProveTimeRow, BatchVerifyTimeRow, BlockFeeComponentRow, L2BlockTimeRow,
    L2GasUsedRow, L2TpsRow, TimeRange,
};
use std::collections::BTreeMap;

/// Determine bucket size based on time range
pub const fn bucket_size_from_range(range: &TimeRange) -> u64 {
    let hours = range.seconds() / 3600;
    if hours <= 1 {
        1
    } else if hours <= 6 {
        5
    } else if hours <= 12 {
        10
    } else if hours <= 24 {
        25
    } else if hours <= 48 {
        50
    } else if hours <= 72 {
        100
    } else {
        250
    }
}

/// Determine bucket size for prove time aggregation. Uses a smaller
/// bucket than [`bucket_size_from_range`] to avoid over-aggregation
/// when prove events are infrequent.
pub const fn prove_bucket_size(range: &TimeRange) -> u64 {
    let base = bucket_size_from_range(range);
    let size = base / 10;
    if size == 0 { 1 } else { size }
}

/// Determine bucket size for verify time aggregation. Uses a much smaller
/// bucket than [`bucket_size_from_range`] to capture more data points
/// since verify events are naturally infrequent.
pub const fn verify_bucket_size(range: &TimeRange) -> u64 {
    let base = bucket_size_from_range(range);
    let size = base / 25;
    if size == 0 { 1 } else { size }
}

/// Aggregate L2 block times by bucket size
pub fn aggregate_l2_block_times(rows: Vec<L2BlockTimeRow>, bucket: u64) -> Vec<L2BlockTimeRow> {
    let bucket = bucket.max(1);
    let mut groups: BTreeMap<u64, Vec<L2BlockTimeRow>> = BTreeMap::new();
    for row in rows {
        groups.entry(row.l2_block_number / bucket).or_default().push(row);
    }
    groups
        .into_iter()
        .map(|(g, mut rs)| {
            rs.sort_by_key(|r| r.l2_block_number);
            let last_time = rs.last().map(|r| r.block_time).unwrap_or_default();
            let (sum, count) = rs
                .iter()
                .filter_map(|r| r.ms_since_prev_block)
                .fold((0u64, 0u64), |(s, c), ms| (s + ms, c + 1));
            let avg = if count > 0 { sum / count } else { 0 };
            L2BlockTimeRow {
                l2_block_number: g * bucket,
                block_time: last_time,
                ms_since_prev_block: Some(avg),
            }
        })
        .collect()
}

/// Aggregate L2 gas used by bucket size
pub fn aggregate_l2_gas_used(rows: Vec<L2GasUsedRow>, bucket: u64) -> Vec<L2GasUsedRow> {
    let bucket = bucket.max(1);
    let mut groups: BTreeMap<u64, Vec<L2GasUsedRow>> = BTreeMap::new();
    for row in rows {
        groups.entry(row.l2_block_number / bucket).or_default().push(row);
    }
    groups
        .into_iter()
        .map(|(g, mut rs)| {
            rs.sort_by_key(|r| r.l2_block_number);
            let last_time = rs.last().map(|r| r.block_time).unwrap_or_default();
            let (sum, count) = rs.iter().fold((0u64, 0u64), |(s, c), r| (s + r.gas_used, c + 1));
            let avg = if count > 0 { sum / count } else { 0 };
            L2GasUsedRow { l2_block_number: g * bucket, block_time: last_time, gas_used: avg }
        })
        .collect()
}

/// Aggregate L2 fee components by bucket size
pub fn aggregate_l2_fee_components(
    rows: Vec<BlockFeeComponentRow>,
    bucket: u64,
) -> Vec<BlockFeeComponentRow> {
    let bucket = bucket.max(1);
    let mut groups: BTreeMap<u64, Vec<BlockFeeComponentRow>> = BTreeMap::new();
    for row in rows {
        groups.entry(row.l2_block_number / bucket).or_default().push(row);
    }
    groups
        .into_iter()
        .map(|(g, rs)| {
            let sum_priority: u128 = rs.iter().map(|r| r.priority_fee).sum();
            let sum_base: u128 = rs.iter().map(|r| r.base_fee).sum();
            let (sum_l1, any): (u128, bool) = rs.iter().fold((0, false), |(s, a), r| {
                (s + r.l1_data_cost.unwrap_or(0), a || r.l1_data_cost.is_some())
            });
            BlockFeeComponentRow {
                l2_block_number: g * bucket,
                priority_fee: sum_priority,
                base_fee: sum_base,
                l1_data_cost: any.then_some(sum_l1),
            }
        })
        .collect()
}

/// Aggregate batch fee components by bucket size
pub fn aggregate_batch_fee_components(
    rows: Vec<BatchFeeComponentRow>,
    bucket: u64,
) -> Vec<BatchFeeComponentRow> {
    let bucket = bucket.max(1);
    let mut groups: BTreeMap<u64, Vec<BatchFeeComponentRow>> = BTreeMap::new();
    for row in rows {
        groups.entry(row.batch_id / bucket).or_default().push(row);
    }
    groups
        .into_iter()
        .map(|(g, rs)| {
            let sum_priority: u128 = rs.iter().map(|r| r.priority_fee).sum();
            let sum_base: u128 = rs.iter().map(|r| r.base_fee).sum();
            let (sum_l1, any_l1): (u128, bool) = rs.iter().fold((0, false), |(s, a), r| {
                (s + r.l1_data_cost.unwrap_or(0), a || r.l1_data_cost.is_some())
            });
            let (sum_prove, any_prove) = rs.iter().fold((0u128, false), |(s, a), r| {
                (s + r.amortized_prove_cost.unwrap_or(0), a || r.amortized_prove_cost.is_some())
            });

            let last_l1 = rs.last().map(|r| r.l1_block_number).unwrap_or_default();
            let last_hash = rs.last().map(|r| r.l1_tx_hash.clone()).unwrap_or_default();
            let last_seq = rs.last().map(|r| r.sequencer.clone()).unwrap_or_default();
            BatchFeeComponentRow {
                batch_id: g * bucket,
                l1_block_number: last_l1,
                l1_tx_hash: last_hash,
                sequencer: last_seq,
                priority_fee: sum_priority,
                base_fee: sum_base,
                l1_data_cost: any_l1.then_some(sum_l1),
                amortized_prove_cost: any_prove.then_some(sum_prove),
            }
        })
        .collect()
}

/// Aggregate L2 TPS by bucket size
pub fn aggregate_l2_tps(rows: Vec<L2TpsRow>, bucket: u64) -> Vec<L2TpsRow> {
    let bucket = bucket.max(1);
    let mut groups: BTreeMap<u64, Vec<L2TpsRow>> = BTreeMap::new();
    for row in rows {
        groups.entry(row.l2_block_number / bucket).or_default().push(row);
    }
    groups
        .into_iter()
        .map(|(g, rs)| {
            let (sum, count) = rs.iter().fold((0f64, 0u64), |(s, c), r| (s + r.tps, c + 1));
            let avg = if count > 0 { sum / count as f64 } else { 0.0 };
            L2TpsRow { l2_block_number: g * bucket, tps: avg }
        })
        .collect()
}

/// Aggregate block transactions by bucket size
pub fn aggregate_block_transactions(
    rows: Vec<BlockTransactionsItem>,
    bucket: u64,
) -> Vec<BlockTransactionsItem> {
    let bucket = bucket.max(1);
    let mut groups: BTreeMap<u64, Vec<BlockTransactionsItem>> = BTreeMap::new();
    for row in rows {
        groups.entry(row.block / bucket).or_default().push(row);
    }
    groups
        .into_iter()
        .map(|(g, mut rs)| {
            rs.sort_by_key(|r| r.block);
            let last_seq = rs.last().map(|r| r.sequencer.clone()).unwrap_or_default();
            let last_time = rs.last().map(|r| r.block_time).unwrap_or_default();
            let (sum, count) = rs.iter().fold((0u64, 0u64), |(s, c), r| (s + r.txs as u64, c + 1));
            let avg = if count > 0 { (sum / count) as u32 } else { 0 };
            BlockTransactionsItem {
                block: g * bucket,
                txs: avg,
                sequencer: last_seq,
                block_time: last_time,
            }
        })
        .collect()
}

/// Aggregate prove times by bucket size
pub fn aggregate_prove_times(rows: Vec<BatchProveTimeRow>, bucket: u64) -> Vec<BatchProveTimeRow> {
    let bucket = bucket.max(1);
    let mut groups: BTreeMap<u64, Vec<BatchProveTimeRow>> = BTreeMap::new();
    for row in rows {
        groups.entry(row.batch_id / bucket).or_default().push(row);
    }
    groups
        .into_iter()
        .map(|(g, rs)| {
            let (sum, count) =
                rs.iter().fold((0u64, 0u64), |(s, c), r| (s + r.seconds_to_prove, c + 1));
            let avg = if count > 0 { sum / count } else { 0 };
            BatchProveTimeRow { batch_id: g * bucket, seconds_to_prove: avg }
        })
        .collect()
}

/// Aggregate verify times by bucket size
pub fn aggregate_verify_times(
    rows: Vec<BatchVerifyTimeRow>,
    bucket: u64,
) -> Vec<BatchVerifyTimeRow> {
    let bucket = bucket.max(1);
    let mut groups: BTreeMap<u64, Vec<BatchVerifyTimeRow>> = BTreeMap::new();
    for row in rows {
        groups.entry(row.batch_id / bucket).or_default().push(row);
    }
    groups
        .into_iter()
        .map(|(g, rs)| {
            let (sum, count) =
                rs.iter().fold((0u64, 0u64), |(s, c), r| (s + r.seconds_to_verify, c + 1));
            let avg = if count > 0 { sum / count } else { 0 };
            BatchVerifyTimeRow { batch_id: g * bucket, seconds_to_verify: avg }
        })
        .collect()
}

/// Aggregate blobs per batch by bucket size
pub fn aggregate_blobs_per_batch(
    rows: Vec<BatchBlobCountRow>,
    bucket: u64,
) -> Vec<AvgBatchBlobCountRow> {
    let bucket = bucket.max(1);
    let mut groups: BTreeMap<u64, Vec<BatchBlobCountRow>> = BTreeMap::new();
    for row in rows {
        groups.entry(row.batch_id / bucket).or_default().push(row);
    }
    groups
        .into_iter()
        .map(|(g, rs)| {
            let sum_blobs: u32 = rs.iter().map(|r| r.blob_count as u32).sum();
            let last_l1_block = rs.last().map(|r| r.l1_block_number).unwrap_or_default();
            let avg_blobs = if rs.is_empty() { 0.0 } else { sum_blobs as f64 / rs.len() as f64 };
            AvgBatchBlobCountRow {
                l1_block_number: last_l1_block,
                batch_id: g * bucket,
                blob_count: avg_blobs,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    // Helper functions for creating test data
    fn create_l2_block_time_row(
        block_num: u64,
        ms_since_prev: Option<u64>,
        time_offset_secs: i64,
    ) -> L2BlockTimeRow {
        L2BlockTimeRow {
            l2_block_number: block_num,
            block_time: Utc::now() + chrono::Duration::seconds(time_offset_secs),
            ms_since_prev_block: ms_since_prev,
        }
    }

    fn create_l2_gas_used_row(
        block_num: u64,
        gas_used: u64,
        time_offset_secs: i64,
    ) -> L2GasUsedRow {
        L2GasUsedRow {
            l2_block_number: block_num,
            block_time: Utc::now() + chrono::Duration::seconds(time_offset_secs),
            gas_used,
        }
    }

    fn create_block_fee_component_row(
        block_num: u64,
        priority_fee: u128,
        base_fee: u128,
        l1_cost: Option<u128>,
    ) -> BlockFeeComponentRow {
        BlockFeeComponentRow {
            l2_block_number: block_num,
            priority_fee,
            base_fee,
            l1_data_cost: l1_cost,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn create_batch_fee_component_row(
        batch_id: u64,
        l1_block_number: u64,
        l1_tx_hash: String,
        sequencer: &str,
        priority_fee: u128,
        base_fee: u128,
        l1_cost: Option<u128>,
        prove_cost: Option<u128>,
    ) -> BatchFeeComponentRow {
        BatchFeeComponentRow {
            batch_id,
            l1_block_number,
            l1_tx_hash,
            sequencer: sequencer.to_owned(),
            priority_fee,
            base_fee,
            l1_data_cost: l1_cost,
            amortized_prove_cost: prove_cost,
        }
    }

    fn create_block_transactions_item(
        block: u64,
        txs: u32,
        sequencer: &str,
        time_offset_secs: i64,
    ) -> BlockTransactionsItem {
        BlockTransactionsItem {
            block,
            txs,
            sequencer: sequencer.to_owned(),
            block_time: Utc::now() + chrono::Duration::seconds(time_offset_secs),
        }
    }

    fn create_l2_tps_row(block_num: u64, tps: f64) -> L2TpsRow {
        L2TpsRow { l2_block_number: block_num, tps }
    }

    fn create_batch_blob_count_row(
        batch_id: u64,
        l1_block_number: u64,
        blob_count: u8,
    ) -> BatchBlobCountRow {
        BatchBlobCountRow { batch_id, l1_block_number, blob_count }
    }

    // Tests for bucket_size_from_range
    #[test]
    fn test_bucket_size_from_range_15_min() {
        let range = TimeRange::Last15Min; // 900 seconds = 0.25 hours
        assert_eq!(bucket_size_from_range(&range), 1);
    }

    #[test]
    fn test_bucket_size_from_range_1_hour() {
        let range = TimeRange::LastHour; // 3600 seconds = 1 hour
        assert_eq!(bucket_size_from_range(&range), 1);
    }

    #[test]
    fn test_bucket_size_from_range_6_hours() {
        let range = TimeRange::Custom(6 * 3600); // 6 hours
        assert_eq!(bucket_size_from_range(&range), 5);
    }

    #[test]
    fn test_bucket_size_from_range_12_hours() {
        let range = TimeRange::Custom(12 * 3600); // 12 hours
        assert_eq!(bucket_size_from_range(&range), 10);
    }

    #[test]
    fn test_bucket_size_from_range_24_hours() {
        let range = TimeRange::Last24Hours; // 24 hours
        assert_eq!(bucket_size_from_range(&range), 25);
    }

    #[test]
    fn test_bucket_size_from_range_48_hours() {
        let range = TimeRange::Custom(48 * 3600); // 48 hours
        assert_eq!(bucket_size_from_range(&range), 50);
    }

    #[test]
    fn test_bucket_size_from_range_72_hours() {
        let range = TimeRange::Custom(72 * 3600); // 72 hours
        assert_eq!(bucket_size_from_range(&range), 100);
    }

    #[test]
    fn test_bucket_size_from_range_7_days() {
        let range = TimeRange::Last7Days; // 7 days
        assert_eq!(bucket_size_from_range(&range), 250);
    }

    #[test]
    fn test_bucket_size_from_range_zero_seconds() {
        let range = TimeRange::Custom(0);
        assert_eq!(bucket_size_from_range(&range), 1);
    }

    #[test]
    fn test_prove_bucket_size_smaller() {
        let range = TimeRange::Custom(6 * 3600); // 6 hours
        assert_eq!(prove_bucket_size(&range), 1); // base 5 / 10 = 0 -> 1
    }

    #[test]
    fn test_verify_bucket_size_smaller() {
        let range = TimeRange::Custom(6 * 3600); // 6 hours
        assert_eq!(verify_bucket_size(&range), 1); // base 5 / 25 = 0 -> 1
    }

    // Tests for aggregate_l2_block_times
    #[test]
    fn test_aggregate_l2_block_times_empty() {
        let rows = vec![];
        let result = aggregate_l2_block_times(rows, 5);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_aggregate_l2_block_times_single_row() {
        let rows = vec![create_l2_block_time_row(10, Some(1000), 0)];
        let result = aggregate_l2_block_times(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].l2_block_number, 10); // 10 / 5 * 5 = 10
        assert_eq!(result[0].ms_since_prev_block, Some(1000));
    }

    #[test]
    fn test_aggregate_l2_block_times_multiple_in_bucket() {
        let rows = vec![
            create_l2_block_time_row(10, Some(1000), 0),
            create_l2_block_time_row(11, Some(2000), 1),
            create_l2_block_time_row(12, Some(3000), 2),
        ];
        let result = aggregate_l2_block_times(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].l2_block_number, 10); // bucket 2 * 5 = 10
        assert_eq!(result[0].ms_since_prev_block, Some(2000)); // (1000 + 2000 + 3000) / 3 = 2000
    }

    #[test]
    fn test_aggregate_l2_block_times_multiple_buckets() {
        let rows = vec![
            create_l2_block_time_row(2, Some(1000), 0),
            create_l2_block_time_row(7, Some(2000), 1),
        ];
        let result = aggregate_l2_block_times(rows, 5);

        assert_eq!(result.len(), 2);
        // First bucket: block 2 -> bucket 0
        assert_eq!(result[0].l2_block_number, 0);
        assert_eq!(result[0].ms_since_prev_block, Some(1000));
        // Second bucket: block 7 -> bucket 1
        assert_eq!(result[1].l2_block_number, 5);
        assert_eq!(result[1].ms_since_prev_block, Some(2000));
    }

    #[test]
    fn test_aggregate_l2_block_times_with_nones() {
        let rows = vec![
            create_l2_block_time_row(10, None, 0),
            create_l2_block_time_row(11, Some(2000), 1),
            create_l2_block_time_row(12, None, 2),
        ];
        let result = aggregate_l2_block_times(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].ms_since_prev_block, Some(2000)); // Only non-None value
    }

    #[test]
    fn test_aggregate_l2_block_times_all_nones() {
        let rows =
            vec![create_l2_block_time_row(10, None, 0), create_l2_block_time_row(11, None, 1)];
        let result = aggregate_l2_block_times(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].ms_since_prev_block, Some(0)); // No valid values, so 0
    }

    #[test]
    fn test_aggregate_l2_block_times_zero_bucket() {
        let rows = vec![create_l2_block_time_row(10, Some(1000), 0)];
        let result = aggregate_l2_block_times(rows, 0);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].l2_block_number, 10); // bucket becomes 1
    }

    // Tests for aggregate_l2_gas_used
    #[test]
    fn test_aggregate_l2_gas_used_empty() {
        let rows = vec![];
        let result = aggregate_l2_gas_used(rows, 5);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_aggregate_l2_gas_used_single_row() {
        let rows = vec![create_l2_gas_used_row(10, 1000000, 0)];
        let result = aggregate_l2_gas_used(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].l2_block_number, 10);
        assert_eq!(result[0].gas_used, 1000000);
    }

    #[test]
    fn test_aggregate_l2_gas_used_averaging() {
        let rows = vec![
            create_l2_gas_used_row(10, 1000000, 0),
            create_l2_gas_used_row(11, 2000000, 1),
            create_l2_gas_used_row(12, 3000000, 2),
        ];
        let result = aggregate_l2_gas_used(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].gas_used, 2000000); // (1M + 2M + 3M) / 3 = 2M
    }

    #[test]
    fn test_aggregate_l2_gas_used_zero_gas() {
        let rows = vec![create_l2_gas_used_row(10, 0, 0), create_l2_gas_used_row(11, 1000000, 1)];
        let result = aggregate_l2_gas_used(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].gas_used, 500000); // (0 + 1M) / 2 = 500K
    }

    // Tests for aggregate_l2_fee_components
    #[test]
    fn test_aggregate_l2_fee_components_empty() {
        let rows = vec![];
        let result = aggregate_l2_fee_components(rows, 5);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_aggregate_l2_fee_components_single_row() {
        let rows = vec![create_block_fee_component_row(10, 1000, 2000, Some(500))];
        let result = aggregate_l2_fee_components(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].l2_block_number, 10);
        assert_eq!(result[0].priority_fee, 1000);
        assert_eq!(result[0].base_fee, 2000);
        assert_eq!(result[0].l1_data_cost, Some(500));
    }

    #[test]
    fn test_aggregate_l2_fee_components_summation() {
        let rows = vec![
            create_block_fee_component_row(10, 1000, 2000, Some(500)),
            create_block_fee_component_row(11, 1500, 2500, Some(600)),
        ];
        let result = aggregate_l2_fee_components(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].priority_fee, 2500); // 1000 + 1500
        assert_eq!(result[0].base_fee, 4500); // 2000 + 2500
        assert_eq!(result[0].l1_data_cost, Some(1100)); // 500 + 600
    }

    #[test]
    fn test_aggregate_l2_fee_components_all_none_l1_cost() {
        let rows = vec![
            create_block_fee_component_row(10, 1000, 2000, None),
            create_block_fee_component_row(11, 1500, 2500, None),
        ];
        let result = aggregate_l2_fee_components(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].l1_data_cost, None); // Should remain None
    }

    #[test]
    fn test_aggregate_l2_fee_components_mixed_l1_cost() {
        let rows = vec![
            create_block_fee_component_row(10, 1000, 2000, None),
            create_block_fee_component_row(11, 1500, 2500, Some(600)),
        ];
        let result = aggregate_l2_fee_components(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].l1_data_cost, Some(600)); // Should be Some due to any=true
    }

    // Tests for aggregate_batch_fee_components
    #[test]
    fn test_aggregate_batch_fee_components_empty() {
        let rows = vec![];
        let result = aggregate_batch_fee_components(rows, 5);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_aggregate_batch_fee_components_single_row() {
        let rows = vec![create_batch_fee_component_row(
            10,
            100,
            "0x0".into(),
            "seq1",
            1000,
            2000,
            Some(500),
            Some(300),
        )];
        let result = aggregate_batch_fee_components(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].batch_id, 10);
        assert_eq!(result[0].l1_block_number, 100);
        assert_eq!(result[0].l1_tx_hash, "0x0");
        assert_eq!(result[0].sequencer, "seq1");
        assert_eq!(result[0].priority_fee, 1000);
        assert_eq!(result[0].base_fee, 2000);
        assert_eq!(result[0].l1_data_cost, Some(500));
        assert_eq!(result[0].amortized_prove_cost, Some(300));
    }

    #[test]
    fn test_aggregate_batch_fee_components_summation() {
        let rows = vec![
            create_batch_fee_component_row(
                10,
                100,
                "0x0".into(),
                "seq1",
                1000,
                2000,
                Some(500),
                Some(300),
            ),
            create_batch_fee_component_row(
                11,
                101,
                "0x1".into(),
                "seq2",
                1500,
                2500,
                Some(600),
                Some(400),
            ),
        ];
        let result = aggregate_batch_fee_components(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].priority_fee, 2500); // 1000 + 1500
        assert_eq!(result[0].base_fee, 4500); // 2000 + 2500
        assert_eq!(result[0].l1_data_cost, Some(1100)); // 500 + 600
        assert_eq!(result[0].amortized_prove_cost, Some(700)); // 300 + 400
        assert_eq!(result[0].l1_block_number, 101); // Last value
        assert_eq!(result[0].l1_tx_hash, "0x1"); // Last value
        assert_eq!(result[0].sequencer, "seq2"); // Last value
    }

    #[test]
    fn test_aggregate_batch_fee_components_mixed_optional_fields() {
        let rows = vec![
            create_batch_fee_component_row(
                10,
                100,
                "0x0".into(),
                "seq1",
                1000,
                2000,
                None,
                Some(300),
            ),
            create_batch_fee_component_row(
                11,
                101,
                "0x1".into(),
                "seq2",
                1500,
                2500,
                Some(600),
                None,
            ),
        ];
        let result = aggregate_batch_fee_components(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].l1_data_cost, Some(600)); // any=true due to second row
        assert_eq!(result[0].amortized_prove_cost, Some(300)); // any=true due to first row
    }

    // Tests for aggregate_block_transactions
    #[test]
    fn test_aggregate_block_transactions_empty() {
        let rows = vec![];
        let result = aggregate_block_transactions(rows, 5);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_aggregate_block_transactions_single_row() {
        let rows = vec![create_block_transactions_item(10, 25, "seq1", 0)];
        let result = aggregate_block_transactions(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].block, 10);
        assert_eq!(result[0].txs, 25);
        assert_eq!(result[0].sequencer, "seq1");
    }

    #[test]
    fn test_aggregate_block_transactions_averaging() {
        let rows = vec![
            create_block_transactions_item(10, 20, "seq1", 0),
            create_block_transactions_item(11, 30, "seq2", 1),
            create_block_transactions_item(12, 40, "seq3", 2),
        ];
        let result = aggregate_block_transactions(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].txs, 30); // (20 + 30 + 40) / 3 = 30
        assert_eq!(result[0].sequencer, "seq3"); // Last value
    }

    #[test]
    fn test_aggregate_block_transactions_zero_txs() {
        let rows = vec![
            create_block_transactions_item(10, 0, "seq1", 0),
            create_block_transactions_item(11, 50, "seq2", 1),
        ];
        let result = aggregate_block_transactions(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].txs, 25); // (0 + 50) / 2 = 25
    }

    #[test]
    fn test_aggregate_block_transactions_multiple_buckets() {
        let rows = vec![
            create_block_transactions_item(2, 20, "seq1", 0),
            create_block_transactions_item(7, 30, "seq2", 1),
        ];
        let result = aggregate_block_transactions(rows, 5);

        assert_eq!(result.len(), 2);
        // First bucket: block 2 -> bucket 0
        assert_eq!(result[0].block, 0);
        assert_eq!(result[0].txs, 20);
        assert_eq!(result[0].sequencer, "seq1");
        // Second bucket: block 7 -> bucket 1
        assert_eq!(result[1].block, 5);
        assert_eq!(result[1].txs, 30);
        assert_eq!(result[1].sequencer, "seq2");
    }

    #[test]
    fn test_aggregate_block_transactions_large_values() {
        let rows = vec![
            create_block_transactions_item(10, u32::MAX - 1, "seq1", 0),
            create_block_transactions_item(11, u32::MAX, "seq2", 1),
        ];
        let result = aggregate_block_transactions(rows, 5);

        assert_eq!(result.len(), 1);
        // Should handle large values correctly: ((2^32-2) + (2^32-1)) / 2 = 2^32 - 1.5 = 2^32 - 2
        // (truncated)
        assert_eq!(result[0].txs, u32::MAX - 1);
    }

    // Edge case tests for all functions
    #[test]
    fn test_all_aggregations_with_bucket_size_1() {
        // Test that bucket size 1 doesn't change block numbers
        let l2_time_rows = vec![
            create_l2_block_time_row(5, Some(1000), 0),
            create_l2_block_time_row(10, Some(2000), 1),
        ];
        let l2_time_result = aggregate_l2_block_times(l2_time_rows, 1);
        assert_eq!(l2_time_result.len(), 2);
        assert_eq!(l2_time_result[0].l2_block_number, 5);
        assert_eq!(l2_time_result[1].l2_block_number, 10);

        let gas_rows =
            vec![create_l2_gas_used_row(5, 1000000, 0), create_l2_gas_used_row(10, 2000000, 1)];
        let gas_result = aggregate_l2_gas_used(gas_rows, 1);
        assert_eq!(gas_result.len(), 2);
        assert_eq!(gas_result[0].l2_block_number, 5);
        assert_eq!(gas_result[1].l2_block_number, 10);
    }

    #[test]
    fn test_bucket_size_larger_than_data_range() {
        // Test when bucket size is larger than the data range
        let rows = vec![
            create_l2_block_time_row(1, Some(1000), 0),
            create_l2_block_time_row(2, Some(2000), 1),
            create_l2_block_time_row(3, Some(3000), 2),
        ];
        let result = aggregate_l2_block_times(rows, 100);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].l2_block_number, 0); // All blocks go to bucket 0
        assert_eq!(result[0].ms_since_prev_block, Some(2000)); // Average
    }

    // Tests for aggregate_l2_tps
    #[test]
    fn test_aggregate_l2_tps_empty() {
        let rows = vec![];
        let result = aggregate_l2_tps(rows, 5);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_aggregate_l2_tps_single_row() {
        let rows = vec![create_l2_tps_row(10, 15.5)];
        let result = aggregate_l2_tps(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].l2_block_number, 10); // 10 / 5 * 5 = 10
        assert_eq!(result[0].tps, 15.5);
    }

    #[test]
    fn test_aggregate_l2_tps_multiple_in_bucket() {
        let rows = vec![
            create_l2_tps_row(10, 10.0),
            create_l2_tps_row(11, 20.0),
            create_l2_tps_row(12, 30.0),
        ];
        let result = aggregate_l2_tps(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].l2_block_number, 10); // bucket 2 * 5 = 10
        assert_eq!(result[0].tps, 20.0); // (10.0 + 20.0 + 30.0) / 3 = 20.0
    }

    #[test]
    fn test_aggregate_l2_tps_multiple_buckets() {
        let rows = vec![create_l2_tps_row(2, 10.0), create_l2_tps_row(7, 30.0)];
        let result = aggregate_l2_tps(rows, 5);

        assert_eq!(result.len(), 2);
        // First bucket: block 2 -> bucket 0
        assert_eq!(result[0].l2_block_number, 0);
        assert_eq!(result[0].tps, 10.0);
        // Second bucket: block 7 -> bucket 1
        assert_eq!(result[1].l2_block_number, 5);
        assert_eq!(result[1].tps, 30.0);
    }

    #[test]
    fn test_aggregate_l2_tps_zero_tps() {
        let rows = vec![create_l2_tps_row(10, 0.0), create_l2_tps_row(11, 10.0)];
        let result = aggregate_l2_tps(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].tps, 5.0); // (0.0 + 10.0) / 2 = 5.0
    }

    #[test]
    fn test_aggregate_l2_tps_zero_bucket() {
        let rows = vec![create_l2_tps_row(10, 15.5)];
        let result = aggregate_l2_tps(rows, 0);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].l2_block_number, 10); // bucket becomes 1
        assert_eq!(result[0].tps, 15.5);
    }

    #[test]
    fn test_aggregate_l2_tps_fractional_values() {
        let rows = vec![
            create_l2_tps_row(10, 1.25),
            create_l2_tps_row(11, 2.75),
            create_l2_tps_row(12, 3.5),
        ];
        let result = aggregate_l2_tps(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].tps, 2.5); // (1.25 + 2.75 + 3.5) / 3 = 2.5
    }

    // Tests for aggregate_prove_times
    #[test]
    fn test_aggregate_prove_times_empty() {
        let rows = vec![];
        let result = aggregate_prove_times(rows, 5);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_aggregate_prove_times_single_row() {
        let rows = vec![BatchProveTimeRow { batch_id: 10, seconds_to_prove: 1000 }];
        let result = aggregate_prove_times(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].batch_id, 10);
        assert_eq!(result[0].seconds_to_prove, 1000);
    }

    #[test]
    fn test_aggregate_prove_times_multiple_rows() {
        let rows = vec![
            BatchProveTimeRow { batch_id: 10, seconds_to_prove: 1000 },
            BatchProveTimeRow { batch_id: 11, seconds_to_prove: 2000 },
            BatchProveTimeRow { batch_id: 12, seconds_to_prove: 3000 },
        ];
        let result = aggregate_prove_times(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].batch_id, 10);
        assert_eq!(result[0].seconds_to_prove, 2000); // (1000 + 2000 + 3000) / 3 = 2000
    }

    #[test]
    fn test_aggregate_prove_times_multiple_buckets() {
        let rows = vec![
            BatchProveTimeRow { batch_id: 2, seconds_to_prove: 1000 },
            BatchProveTimeRow { batch_id: 7, seconds_to_prove: 2000 },
        ];
        let result = aggregate_prove_times(rows, 5);

        assert_eq!(result.len(), 2);
        // First bucket: batch 2 -> bucket 0
        assert_eq!(result[0].batch_id, 0);
        assert_eq!(result[0].seconds_to_prove, 1000);
        // Second bucket: batch 7 -> bucket 1
        assert_eq!(result[1].batch_id, 5);
        assert_eq!(result[1].seconds_to_prove, 2000);
    }

    #[test]
    fn test_aggregate_prove_times_zero_seconds() {
        let rows = vec![BatchProveTimeRow { batch_id: 10, seconds_to_prove: 0 }];
        let result = aggregate_prove_times(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].batch_id, 10);
        assert_eq!(result[0].seconds_to_prove, 0);
    }

    #[test]
    fn test_aggregate_prove_times_zero_bucket() {
        let rows = vec![
            BatchProveTimeRow { batch_id: 10, seconds_to_prove: 60 },
            BatchProveTimeRow { batch_id: 20, seconds_to_prove: 120 },
        ];
        let result = aggregate_prove_times(rows, 0);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].batch_id, 10);
        assert_eq!(result[0].seconds_to_prove, 60);
    }

    // Tests for aggregate_verify_times
    #[test]
    fn test_aggregate_verify_times_empty() {
        let rows = vec![];
        let result = aggregate_verify_times(rows, 5);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_aggregate_verify_times_single_row() {
        let rows = vec![BatchVerifyTimeRow { batch_id: 10, seconds_to_verify: 60 }];
        let result = aggregate_verify_times(rows, 5);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].batch_id, 10);
        assert_eq!(result[0].seconds_to_verify, 60);
    }

    #[test]
    fn test_aggregate_verify_times_multiple_rows() {
        let rows = vec![
            BatchVerifyTimeRow { batch_id: 10, seconds_to_verify: 60 },
            BatchVerifyTimeRow { batch_id: 12, seconds_to_verify: 120 },
            BatchVerifyTimeRow { batch_id: 14, seconds_to_verify: 180 },
        ];
        let result = aggregate_verify_times(rows, 5);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].batch_id, 10);
        assert_eq!(result[0].seconds_to_verify, 120); // (60 + 120 + 180) / 3 = 120
    }

    #[test]
    fn test_aggregate_verify_times_multiple_buckets() {
        let rows = vec![
            BatchVerifyTimeRow { batch_id: 10, seconds_to_verify: 60 },
            BatchVerifyTimeRow { batch_id: 12, seconds_to_verify: 120 },
            BatchVerifyTimeRow { batch_id: 20, seconds_to_verify: 180 },
            BatchVerifyTimeRow { batch_id: 22, seconds_to_verify: 240 },
        ];
        let result = aggregate_verify_times(rows, 5);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].batch_id, 10);
        assert_eq!(result[0].seconds_to_verify, 90); // (60 + 120) / 2 = 90
        assert_eq!(result[1].batch_id, 20);
        assert_eq!(result[1].seconds_to_verify, 210); // (180 + 240) / 2 = 210
    }

    #[test]
    fn test_aggregate_verify_times_zero_seconds() {
        let rows = vec![
            BatchVerifyTimeRow { batch_id: 10, seconds_to_verify: 0 },
            BatchVerifyTimeRow { batch_id: 12, seconds_to_verify: 60 },
        ];
        let result = aggregate_verify_times(rows, 5);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].batch_id, 10);
        assert_eq!(result[0].seconds_to_verify, 30); // (0 + 60) / 2 = 30
    }

    #[test]
    fn test_aggregate_verify_times_zero_bucket() {
        let rows = vec![
            BatchVerifyTimeRow { batch_id: 10, seconds_to_verify: 60 },
            BatchVerifyTimeRow { batch_id: 20, seconds_to_verify: 120 },
        ];
        let result = aggregate_verify_times(rows, 0);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].batch_id, 10);
        assert_eq!(result[0].seconds_to_verify, 60);
    }

    // Tests for aggregate_blobs_per_batch
    #[test]
    fn test_aggregate_blobs_per_batch_empty() {
        let rows = vec![];
        let result = aggregate_blobs_per_batch(rows, 5);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_aggregate_blobs_per_batch_single_row() {
        let rows = vec![create_batch_blob_count_row(10, 100, 3)];
        let result = aggregate_blobs_per_batch(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].batch_id, 10);
        assert_eq!(result[0].l1_block_number, 100);
        assert_eq!(result[0].blob_count, 3.0);
    }

    #[test]
    fn test_aggregate_blobs_per_batch_multiple_in_bucket() {
        let rows = vec![
            create_batch_blob_count_row(10, 100, 2),
            create_batch_blob_count_row(11, 101, 4),
            create_batch_blob_count_row(12, 102, 6),
        ];
        let result = aggregate_blobs_per_batch(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].batch_id, 10); // bucket 2 * 5 = 10
        assert_eq!(result[0].l1_block_number, 102); // last l1_block_number
        assert_eq!(result[0].blob_count, 4.0); // (2 + 4 + 6) / 3 = 4
    }

    #[test]
    fn test_aggregate_blobs_per_batch_multiple_buckets() {
        let rows =
            vec![create_batch_blob_count_row(2, 100, 2), create_batch_blob_count_row(7, 105, 6)];
        let result = aggregate_blobs_per_batch(rows, 5);

        assert_eq!(result.len(), 2);
        // First bucket: batch 2 -> bucket 0
        assert_eq!(result[0].batch_id, 0);
        assert_eq!(result[0].l1_block_number, 100);
        assert_eq!(result[0].blob_count, 2.0);
        // Second bucket: batch 7 -> bucket 1
        assert_eq!(result[1].batch_id, 5);
        assert_eq!(result[1].l1_block_number, 105);
        assert_eq!(result[1].blob_count, 6.0);
    }

    #[test]
    fn test_aggregate_blobs_per_batch_zero_blobs() {
        let rows =
            vec![create_batch_blob_count_row(10, 100, 0), create_batch_blob_count_row(11, 101, 4)];
        let result = aggregate_blobs_per_batch(rows, 5);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].blob_count, 2.0); // (0 + 4) / 2 = 2
    }

    #[test]
    fn test_aggregate_blobs_per_batch_zero_bucket() {
        let rows =
            vec![create_batch_blob_count_row(10, 100, 3), create_batch_blob_count_row(20, 200, 5)];
        let result = aggregate_blobs_per_batch(rows, 0);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].batch_id, 10);
        assert_eq!(result[0].blob_count, 3.0);
        assert_eq!(result[1].batch_id, 20);
        assert_eq!(result[1].blob_count, 5.0);
    }

    #[test]
    fn test_aggregate_blobs_per_batch_large_values() {
        let rows = vec![
            create_batch_blob_count_row(1000, 5000, 255), // max u8 value
            create_batch_blob_count_row(1001, 5001, 200),
            create_batch_blob_count_row(1002, 5002, 100),
        ];
        let result = aggregate_blobs_per_batch(rows, 10);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].batch_id, 1000);
        assert_eq!(result[0].l1_block_number, 5002);
        assert_eq!(result[0].blob_count, 185.0); // (255 + 200 + 100) / 3 = 185
    }
}
