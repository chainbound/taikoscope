//! Schema definitions for `ClickHouse` tables

/// Table schema definition
#[derive(Debug)]
pub struct TableSchema {
    /// Table name
    pub name: &'static str,
    /// Column definitions
    pub columns: &'static str,
    /// Order by clause
    pub order_by: &'static str,
}

/// Names of all tables
pub const TABLES: &[&str] = &[
    "l1_head_events",
    "preconf_data",
    "l2_head_events",
    "batches",
    "proved_batches",
    "l2_reorgs",
    "forced_inclusion_processed",
    "verified_batches",
    "slashing_events",
    "l1_data_costs",
];

/// Names of all materialized views
pub const VIEWS: &[&str] = &[
    "batch_prove_times_mv",
    "batch_verify_times_mv",
    "hourly_avg_prove_times_mv",
    "hourly_avg_verify_times_mv",
    "hourly_l2_metrics_mv",
    "hourly_batch_metrics_mv",
    "daily_avg_prove_times_mv",
    "daily_avg_verify_times_mv",
    "daily_l2_metrics_mv",
    "daily_batch_metrics_mv",
];

/// Schema definitions for tables
pub const TABLE_SCHEMAS: &[TableSchema] = &[
    TableSchema {
        name: "l1_head_events",
        columns: "l1_block_number UInt64,
                 block_hash FixedString(32),
                 slot UInt64,
                 block_ts UInt64,
                 inserted_at DateTime64(3) DEFAULT now64()",
        order_by: "l1_block_number",
    },
    TableSchema {
        name: "preconf_data",
        columns: "slot UInt64,
                 candidates Array(FixedString(20)),
                 current_operator Nullable(FixedString(20)),
                 next_operator Nullable(FixedString(20)),
                 inserted_at DateTime64(3) DEFAULT now64()",
        order_by: "slot",
    },
    TableSchema {
        name: "l2_head_events",
        columns: "l2_block_number UInt64,
                 block_hash FixedString(32),
                 block_ts UInt64,
                 sum_gas_used UInt128,
                 sum_tx UInt32,
                 sum_priority_fee UInt128,
                 sum_base_fee UInt128,
                 sequencer FixedString(20),
                 inserted_at DateTime64(3) DEFAULT now64()",
        order_by: "l2_block_number",
    },
    TableSchema {
        name: "batches",
        columns: "l1_block_number UInt64,
                 batch_id UInt64,
                 batch_size UInt16,
                 proposer_addr FixedString(20),
                 blob_count UInt8,
                 blob_total_bytes UInt32,
                 inserted_at DateTime64(3) DEFAULT now64()",
        order_by: "l1_block_number, batch_id",
    },
    TableSchema {
        name: "proved_batches",
        columns: "l1_block_number UInt64,
                 batch_id UInt64,
                 verifier_addr FixedString(20),
                 parent_hash FixedString(32),
                 block_hash FixedString(32),
                 state_root FixedString(32),
                 inserted_at DateTime64(3) DEFAULT now64()",
        order_by: "l1_block_number, batch_id",
    },
    TableSchema {
        name: "l2_reorgs",
        columns: "l2_block_number UInt64,
                 depth UInt16,
                 inserted_at DateTime64(3) DEFAULT now64()",
        order_by: "inserted_at",
    },
    TableSchema {
        name: "forced_inclusion_processed",
        columns: "blob_hash FixedString(32),
                 inserted_at DateTime64(3) DEFAULT now64()",
        order_by: "inserted_at",
    },
    TableSchema {
        name: "verified_batches",
        columns: "l1_block_number UInt64,
                 batch_id UInt64,
                 block_hash FixedString(32),
                 inserted_at DateTime64(3) DEFAULT now64()",
        order_by: "l1_block_number, batch_id",
    },
    TableSchema {
        name: "slashing_events",
        columns: "l1_block_number UInt64,
                 validator_addr FixedString(20),
                 inserted_at DateTime64(3) DEFAULT now64()",
        order_by: "l1_block_number, validator_addr",
    },
    TableSchema {
        name: "l1_data_costs",
        columns: "l1_block_number UInt64,
                 l2_block_number UInt64,
                 cost UInt128,
                 inserted_at DateTime64(3) DEFAULT now64()",
        order_by: "l1_block_number",
    },
];
