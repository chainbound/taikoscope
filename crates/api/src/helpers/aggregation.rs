//! Data aggregation utilities

use api_types::BlockTransactionsItem;

use clickhouse_lib::{
    BatchFeeComponentRow, BlockFeeComponentRow, L2BlockTimeRow, L2GasUsedRow, TimeRange,
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
            let (sum_l1, any): (u128, bool) = rs.iter().fold((0, false), |(s, a), r| {
                (s + r.l1_data_cost.unwrap_or(0), a || r.l1_data_cost.is_some())
            });
            let last_l1 = rs.last().map(|r| r.l1_block_number).unwrap_or_default();
            BatchFeeComponentRow {
                batch_id: g * bucket,
                l1_block_number: last_l1,
                priority_fee: sum_priority,
                base_fee: sum_base,
                l1_data_cost: any.then_some(sum_l1),
            }
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
