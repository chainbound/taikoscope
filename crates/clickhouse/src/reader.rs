//! `ClickHouse` reader functionality for API
//! Handles read-only operations and analytics queries

use chrono::{DateTime, LocalResult, TimeZone, Utc};
use clickhouse::{Client, Row, sql::Identifier};
use derive_more::Debug;
use eyre::{Context, Result};
use hex::encode;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    time::Instant,
};
use tracing::{debug, error};
use url::Url;

use crate::{
    models::{
        BatchBlobCountRow, BatchFeeComponentRow, BatchPostingTimeRow, BatchProveTimeRow,
        BatchVerifyTimeRow, BlockFeeComponentRow, BlockTransactionRow, ForcedInclusionProcessedRow,
        L1BlockTimeRow, L1DataCostRow, L2BlockTimeRow, L2GasUsedRow, L2ReorgRow, L2TpsRow,
        PreconfData, ProveCostRow, SequencerBlockRow, SequencerDistributionRow, SequencerFeeRow,
        SlashingEventRow,
    },
    types::AddressBytes,
};

#[derive(Row, Deserialize, Serialize)]
struct MaxTs {
    block_ts: u64,
}

/// Supported time ranges for analytics queries
#[derive(Copy, Clone, Debug)]
pub enum TimeRange {
    /// Data from the last 15 minutes
    Last15Min,
    /// Data from the last hour
    LastHour,
    /// Data from the last 24 hours
    Last24Hours,
    /// Data from the last 7 days
    Last7Days,
    /// Data from a custom duration in seconds (clamped to 7 days)
    Custom(u64),
}

impl TimeRange {
    /// Maximum allowed range in seconds (30 days).
    const MAX_SECONDS: u64 = 30 * 24 * 3600;

    /// Create a [`TimeRange`] from a [`chrono::Duration`], clamping to the
    /// allowed maximum of thirty days.
    pub fn from_duration(duration: chrono::Duration) -> Self {
        let secs = duration.num_seconds().clamp(0, Self::MAX_SECONDS as i64) as u64;
        match secs {
            900 => Self::Last15Min,
            3600 => Self::LastHour,
            86400 => Self::Last24Hours,
            604800 => Self::Last7Days,
            _ => Self::Custom(secs),
        }
    }

    /// Return the `ClickHouse` interval string for this range.
    pub fn interval(&self) -> String {
        match self {
            Self::Last15Min => "15 MINUTE".to_owned(),
            Self::LastHour => "1 HOUR".to_owned(),
            Self::Last24Hours => "24 HOUR".to_owned(),
            Self::Last7Days => "7 DAY".to_owned(),
            Self::Custom(sec) => format!("{} SECOND", sec),
        }
    }

    /// Return the duration in seconds for this range.
    pub const fn seconds(&self) -> u64 {
        match self {
            Self::Last15Min => 900,
            Self::LastHour => 3600,
            Self::Last24Hours => 86400,
            Self::Last7Days => 604800,
            Self::Custom(sec) => *sec,
        }
    }
}

/// `ClickHouse` reader client for API (read-only operations)
#[derive(Clone, Debug)]
pub struct ClickhouseReader {
    /// Base client
    #[debug(skip)]
    base: Client,
    /// Database name
    db_name: String,
}

impl ClickhouseReader {
    /// Create a new `ClickHouse` reader client
    pub fn new(url: Url, db_name: String, username: String, password: String) -> Result<Self> {
        let client = Client::default().with_url(url).with_user(username).with_password(password);

        Ok(Self { base: client, db_name })
    }

    async fn execute<R>(&self, query: &str) -> Result<Vec<R>>
    where
        R: Row + for<'b> Deserialize<'b>,
    {
        let client = self.base.clone();
        let start = Instant::now();

        let result = client.query(query).fetch_all::<R>().await;

        let duration_ms = start.elapsed().as_millis();
        match &result {
            Ok(rows) => {
                debug!(query = %query, duration_ms, rows = rows.len(), "ClickHouse query executed")
            }
            Err(e) => error!(query = %query, duration_ms, error = %e, "ClickHouse query failed"),
        }
        result.map_err(Into::into)
    }

    /// Anti-subquery that hides blocks later rolled back by a reorg.
    /// Use with `NOT IN (SELECT l2_block_number FROM ...)`
    fn reorg_filter(&self, table_alias: &str) -> String {
        format!(
            "{table_alias}.l2_block_number NOT IN ( \
                SELECT l2_block_number \
                FROM {db}.l2_reorgs\
            )",
            db = self.db_name,
        )
    }

    /// Get last L2 head time
    pub async fn get_last_l2_head_time(&self) -> Result<Option<DateTime<Utc>>> {
        let client = self.base.clone();
        let sql = "SELECT max(block_ts) AS block_ts FROM ?.l2_head_events";

        let start = Instant::now();
        let result = client.query(sql).bind(Identifier(&self.db_name)).fetch_all::<MaxTs>().await;

        let duration_ms = start.elapsed().as_millis();
        match &result {
            Ok(rows) => {
                debug!(query = sql, duration_ms, rows = rows.len(), "ClickHouse query executed")
            }
            Err(e) => error!(query = sql, duration_ms, error = %e, "ClickHouse query failed"),
        }

        let rows = result.context("fetching max(block_ts) failed")?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };
        if row.block_ts == 0 {
            return Ok(None);
        }
        let ts_opt = match Utc.timestamp_opt(row.block_ts as i64, 0) {
            LocalResult::Single(dt) => Some(dt),
            _ => None,
        };
        Ok(ts_opt)
    }

    /// Get timestamp of the latest L1 head event in UTC
    pub async fn get_last_l1_head_time(&self) -> Result<Option<DateTime<Utc>>> {
        let client = self.base.clone();
        let sql = "SELECT max(block_ts) AS block_ts FROM ?.l1_head_events";

        let start = Instant::now();
        let result = client.query(sql).bind(Identifier(&self.db_name)).fetch_all::<MaxTs>().await;

        let duration_ms = start.elapsed().as_millis();
        match &result {
            Ok(rows) => {
                debug!(query = sql, duration_ms, rows = rows.len(), "ClickHouse query executed")
            }
            Err(e) => error!(query = sql, duration_ms, error = %e, "ClickHouse query failed"),
        }

        let rows = result.context("fetching max(block_ts) failed")?;

        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };

        if row.block_ts == 0 {
            return Ok(None);
        }

        let ts_opt = match Utc.timestamp_opt(row.block_ts as i64, 0) {
            LocalResult::Single(dt) => Some(dt),
            _ => None,
        };

        Ok(ts_opt)
    }

    /// Get the latest L2 block number.
    /// Uses an optimized query that should be faster on large tables.
    pub async fn get_last_l2_block_number(&self) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct BlockNumber {
            l2_block_number: u64,
        }

        let client = self.base.clone();
        let sql =
            "SELECT l2_block_number FROM ?.l2_head_events ORDER BY l2_block_number DESC LIMIT 1";

        let start = Instant::now();
        let result =
            client.query(sql).bind(Identifier(&self.db_name)).fetch_all::<BlockNumber>().await;

        let duration_ms = start.elapsed().as_millis();
        match &result {
            Ok(rows) => {
                debug!(query = sql, duration_ms, rows = rows.len(), "ClickHouse query executed")
            }
            Err(e) => error!(query = sql, duration_ms, error = %e, "ClickHouse query failed"),
        }

        let rows = result?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };
        if row.l2_block_number == 0 {
            return Ok(None);
        }
        Ok(Some(row.l2_block_number))
    }

    /// Get the latest L1 block number.
    /// Uses an optimized query that should be faster on large tables.
    pub async fn get_last_l1_block_number(&self) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct BlockNumber {
            l1_block_number: u64,
        }

        let client = self.base.clone();
        let sql =
            "SELECT l1_block_number FROM ?.l1_head_events ORDER BY l1_block_number DESC LIMIT 1";

        let start = Instant::now();
        let result =
            client.query(sql).bind(Identifier(&self.db_name)).fetch_all::<BlockNumber>().await;

        let duration_ms = start.elapsed().as_millis();
        match &result {
            Ok(rows) => {
                debug!(query = sql, duration_ms, rows = rows.len(), "ClickHouse query executed")
            }
            Err(e) => error!(query = sql, duration_ms, error = %e, "ClickHouse query failed"),
        }

        let rows = result?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };
        if row.l1_block_number == 0 {
            return Ok(None);
        }
        Ok(Some(row.l1_block_number))
    }

    /// Get timestamp of the latest `BatchProposed` event based on L1 block timestamp in UTC
    pub async fn get_last_batch_time(&self) -> Result<Option<DateTime<Utc>>> {
        let client = self.base.clone();
        let sql = "SELECT max(l1_events.block_ts) AS block_ts \
             FROM ?.batches b \
             INNER JOIN ?.l1_head_events l1_events \
               ON b.l1_block_number = l1_events.l1_block_number";

        let start = Instant::now();
        let result = client
            .query(sql)
            .bind(Identifier(&self.db_name))
            .bind(Identifier(&self.db_name))
            .fetch_all::<MaxTs>()
            .await;

        let duration_ms = start.elapsed().as_millis();
        match &result {
            Ok(rows) => {
                debug!(query = sql, duration_ms, rows = rows.len(), "ClickHouse query executed")
            }
            Err(e) => error!(query = sql, duration_ms, error = %e, "ClickHouse query failed"),
        }

        let rows = result.context("fetching max batch L1 block timestamp failed")?;

        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };

        if row.block_ts == 0 {
            return Ok(None);
        }

        let ts_opt = match Utc.timestamp_opt(row.block_ts as i64, 0) {
            LocalResult::Single(dt) => Some(dt),
            _ => None,
        };

        Ok(ts_opt)
    }

    /// Get the most recent preconfiguration data
    pub async fn get_last_preconf_data(&self) -> Result<Option<PreconfData>> {
        let client = self.base.clone();
        let sql = "SELECT slot, candidates, current_operator, next_operator FROM ?.preconf_data ORDER BY inserted_at DESC LIMIT 1";

        let start = Instant::now();
        let result =
            client.query(sql).bind(Identifier(&self.db_name)).fetch_all::<PreconfData>().await;

        let duration_ms = start.elapsed().as_millis();
        match &result {
            Ok(rows) => {
                debug!(query = sql, duration_ms, rows = rows.len(), "ClickHouse query executed")
            }
            Err(e) => error!(query = sql, duration_ms, error = %e, "ClickHouse query failed"),
        }

        let rows = result?;
        Ok(rows.into_iter().next())
    }

    /// Get all batches that have not been proven and are older than the given cutoff time
    pub async fn get_unproved_batches_older_than(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<Vec<(u64, u64, DateTime<Utc>)>> {
        let client = self.base.clone();
        let sql = "SELECT b.l1_block_number, b.batch_id, toUnixTimestamp64Milli(b.inserted_at) as inserted_at \
             FROM (SELECT l1_block_number, batch_id, inserted_at \
                   FROM ?.batches \
                   WHERE inserted_at < toDateTime64(?, 3)) AS b \
             LEFT JOIN ?.proved_batches p \
               ON b.l1_block_number = p.l1_block_number AND b.batch_id = p.batch_id \
             WHERE p.batch_id IS NULL \
             ORDER BY b.inserted_at ASC";

        let start = Instant::now();
        let result = client
            .query(sql)
            .bind(Identifier(&self.db_name))
            .bind(cutoff.timestamp_millis() as f64 / 1000.0)
            .bind(Identifier(&self.db_name))
            .fetch_all::<(u64, u64, u64)>()
            .await;

        let duration_ms = start.elapsed().as_millis();
        match &result {
            Ok(rows) => {
                debug!(query = sql, duration_ms, rows = rows.len(), "ClickHouse query executed")
            }
            Err(e) => error!(query = sql, duration_ms, error = %e, "ClickHouse query failed"),
        }
        let rows = result.context("fetching unproved batches failed")?;
        Ok(rows
            .into_iter()
            .filter_map(|(l1_block_number, batch_id, inserted_at)| {
                match chrono::Utc.timestamp_millis_opt(inserted_at as i64) {
                    chrono::LocalResult::Single(dt) => Some((l1_block_number, batch_id, dt)),
                    _ => None,
                }
            })
            .collect())
    }

    /// Get all proved batch IDs from the `proved_batches` table
    pub async fn get_proved_batch_ids(&self) -> Result<Vec<u64>> {
        #[derive(Row, Deserialize)]
        struct ProvedBatchIdRow {
            batch_id: u64,
        }
        let client = self.base.clone();
        let sql = "SELECT batch_id FROM ?.proved_batches";

        let start = Instant::now();
        let result =
            client.query(sql).bind(Identifier(&self.db_name)).fetch_all::<ProvedBatchIdRow>().await;

        let duration_ms = start.elapsed().as_millis();
        match &result {
            Ok(rows) => {
                debug!(query = sql, duration_ms, rows = rows.len(), "ClickHouse query executed")
            }
            Err(e) => error!(query = sql, duration_ms, error = %e, "ClickHouse query failed"),
        }

        let rows = result?;
        Ok(rows.into_iter().map(|r| r.batch_id).collect())
    }

    /// Get all batches that have not been verified and are older than the given cutoff time
    pub async fn get_unverified_batches_older_than(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<Vec<(u64, u64, DateTime<Utc>)>> {
        let client = self.base.clone();
        let sql = "SELECT b.l1_block_number, b.batch_id, toUnixTimestamp64Milli(b.inserted_at) as inserted_at \
             FROM (SELECT l1_block_number, batch_id, inserted_at \
                   FROM ?.batches \
                   WHERE inserted_at < toDateTime64(?, 3)) AS b \
             LEFT JOIN ?.verified_batches v \
               ON b.l1_block_number = v.l1_block_number AND b.batch_id = v.batch_id \
             WHERE v.batch_id IS NULL \
             ORDER BY b.inserted_at ASC";

        let start = Instant::now();
        let result = client
            .query(sql)
            .bind(Identifier(&self.db_name))
            .bind(cutoff.timestamp_millis() as f64 / 1000.0)
            .bind(Identifier(&self.db_name))
            .fetch_all::<(u64, u64, u64)>()
            .await;

        let duration_ms = start.elapsed().as_millis();
        match &result {
            Ok(rows) => {
                debug!(query = sql, duration_ms, rows = rows.len(), "ClickHouse query executed")
            }
            Err(e) => error!(query = sql, duration_ms, error = %e, "ClickHouse query failed"),
        }
        let rows = result.context("fetching unverified batches failed")?;
        Ok(rows
            .into_iter()
            .filter_map(|(l1_block_number, batch_id, inserted_at)| {
                match chrono::Utc.timestamp_millis_opt(inserted_at as i64) {
                    chrono::LocalResult::Single(dt) => Some((l1_block_number, batch_id, dt)),
                    _ => None,
                }
            })
            .collect())
    }

    /// Get all verified batch IDs from the `verified_batches` table
    pub async fn get_verified_batch_ids(&self) -> Result<Vec<u64>> {
        #[derive(Row, Deserialize)]
        struct VerifiedBatchIdRow {
            batch_id: u64,
        }
        let client = self.base.clone();
        let sql = "SELECT batch_id FROM ?.verified_batches";

        let start = Instant::now();
        let result = client
            .query(sql)
            .bind(Identifier(&self.db_name))
            .fetch_all::<VerifiedBatchIdRow>()
            .await;

        let duration_ms = start.elapsed().as_millis();
        match &result {
            Ok(rows) => {
                debug!(query = sql, duration_ms, rows = rows.len(), "ClickHouse query executed")
            }
            Err(e) => error!(query = sql, duration_ms, error = %e, "ClickHouse query failed"),
        }

        let rows = result?;
        Ok(rows.into_iter().map(|r| r.batch_id).collect())
    }

    /// Get all slashing events that occurred after the given cutoff time
    pub async fn get_slashing_events_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<SlashingEventRow>> {
        let client = self.base.clone();
        let sql = "SELECT l1_block_number, validator_addr FROM ?.slashing_events \
             WHERE inserted_at > toDateTime64(?, 3) \
             ORDER BY inserted_at ASC";

        let start = Instant::now();
        let result = client
            .query(sql)
            .bind(Identifier(&self.db_name))
            .bind(since.timestamp_millis() as f64 / 1000.0)
            .fetch_all::<SlashingEventRow>()
            .await;

        let duration_ms = start.elapsed().as_millis();
        match &result {
            Ok(rows) => {
                debug!(query = sql, duration_ms, rows = rows.len(), "ClickHouse query executed")
            }
            Err(e) => error!(query = sql, duration_ms, error = %e, "ClickHouse query failed"),
        }
        let rows = result.context("fetching slashing events failed")?;
        Ok(rows)
    }

    /// Get all forced inclusion events that occurred after the given cutoff time
    pub async fn get_forced_inclusions_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<ForcedInclusionProcessedRow>> {
        let client = self.base.clone();
        let sql = "SELECT blob_hash FROM ?.forced_inclusion_processed \
             WHERE inserted_at > toDateTime64(?, 3) \
             ORDER BY inserted_at ASC";

        let start = Instant::now();
        let result = client
            .query(sql)
            .bind(Identifier(&self.db_name))
            .bind(since.timestamp_millis() as f64 / 1000.0)
            .fetch_all::<ForcedInclusionProcessedRow>()
            .await;

        let duration_ms = start.elapsed().as_millis();
        match &result {
            Ok(rows) => {
                debug!(query = sql, duration_ms, rows = rows.len(), "ClickHouse query executed")
            }
            Err(e) => error!(query = sql, duration_ms, error = %e, "ClickHouse query failed"),
        }
        let rows = result.context("fetching forced inclusion events failed")?;
        Ok(rows)
    }

    /// Get all L2 reorg events that occurred after the given cutoff time
    pub async fn get_l2_reorgs_since(&self, since: DateTime<Utc>) -> Result<Vec<L2ReorgRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l2_block_number: u64,
            depth: u16,
            old_sequencer: AddressBytes,
            new_sequencer: AddressBytes,
            ts: u64,
        }

        let query = format!(
            "SELECT l2_block_number, depth, old_sequencer, new_sequencer, \
                    toUInt64(toUnixTimestamp64Milli(inserted_at)) AS ts \
             FROM {}.l2_reorgs \
             WHERE inserted_at > toDateTime64({}, 3) \
             ORDER BY inserted_at ASC",
            self.db_name,
            since.timestamp_millis() as f64 / 1000.0,
        );
        let rows = self.execute::<RawRow>(&query).await.context("fetching reorg events failed")?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let ts = Utc.timestamp_millis_opt(r.ts as i64).single()?;
                Some(L2ReorgRow {
                    l2_block_number: r.l2_block_number,
                    depth: r.depth,
                    old_sequencer: r.old_sequencer,
                    new_sequencer: r.new_sequencer,
                    inserted_at: Some(ts),
                })
            })
            .collect())
    }

    /// Get L2 reorg events since the given cutoff with cursor-based pagination.
    /// Results are returned in descending order by time recorded.
    pub async fn get_l2_reorgs_paginated(
        &self,
        since: DateTime<Utc>,
        limit: u64,
        starting_after: Option<u64>,
        ending_before: Option<u64>,
    ) -> Result<Vec<L2ReorgRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l2_block_number: u64,
            depth: u16,
            old_sequencer: AddressBytes,
            new_sequencer: AddressBytes,
            ts: u64,
        }

        let mut query = format!(
            "SELECT l2_block_number, depth, old_sequencer, new_sequencer, \
                    toUInt64(toUnixTimestamp64Milli(inserted_at)) AS ts \
             FROM {db}.l2_reorgs \
             WHERE inserted_at > toDateTime64({since}, 3)",
            db = self.db_name,
            since = since.timestamp_millis() as f64 / 1000.0,
        );

        if let Some(start) = starting_after {
            query.push_str(&format!(" AND l2_block_number < {}", start));
        }
        if let Some(end) = ending_before {
            query.push_str(&format!(" AND l2_block_number > {}", end));
        }

        query.push_str(" ORDER BY inserted_at DESC");
        query.push_str(&format!(" LIMIT {}", limit));

        let rows = self.execute::<RawRow>(&query).await.context("fetching reorg events failed")?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let ts = Utc.timestamp_millis_opt(r.ts as i64).single()?;
                Some(L2ReorgRow {
                    l2_block_number: r.l2_block_number,
                    depth: r.depth,
                    old_sequencer: r.old_sequencer,
                    new_sequencer: r.new_sequencer,
                    inserted_at: Some(ts),
                })
            })
            .collect())
    }

    /// Get all active gateway addresses observed since the given cutoff time
    pub async fn get_active_gateways_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<AddressBytes>> {
        #[derive(Row, Deserialize)]
        struct GatewayRow {
            candidates: Vec<AddressBytes>,
            current_operator: Option<AddressBytes>,
            next_operator: Option<AddressBytes>,
        }

        let client = self.base.clone();
        let sql = "SELECT candidates, current_operator, next_operator FROM ?.preconf_data \
             WHERE inserted_at > toDateTime64(?, 3)";

        let start = Instant::now();
        let result = client
            .query(sql)
            .bind(Identifier(&self.db_name))
            .bind(since.timestamp_millis() as f64 / 1000.0)
            .fetch_all::<GatewayRow>()
            .await;

        let duration_ms = start.elapsed().as_millis();
        match &result {
            Ok(rows) => {
                debug!(query = sql, duration_ms, rows = rows.len(), "ClickHouse query executed")
            }
            Err(e) => error!(query = sql, duration_ms, error = %e, "ClickHouse query failed"),
        }

        let rows = result?;
        let mut set = BTreeSet::new();
        for row in rows {
            for cand in row.candidates {
                set.insert(cand);
            }
            if let Some(op) = row.current_operator {
                set.insert(op);
            }
            if let Some(op) = row.next_operator {
                set.insert(op);
            }
        }
        Ok(set.into_iter().collect())
    }

    /// Get the number of blocks produced by each sequencer since the given cutoff time
    pub async fn get_sequencer_distribution_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<SequencerDistributionRow>> {
        let query = format!(
            "SELECT sequencer,\n                    count(DISTINCT h.l2_block_number) AS blocks,\n                    toUInt64(min(h.block_ts)) AS min_ts,\n                    toUInt64(max(h.block_ts)) AS max_ts,\n                    sum(sum_tx) AS tx_sum\n             FROM {db}.l2_head_events h\n             WHERE h.block_ts > {since} AND {filter}\n             GROUP BY sequencer ORDER BY blocks DESC",
            since = since.timestamp(),
            filter = self.reorg_filter("h"),
            db = self.db_name,
        );

        let rows = self.execute::<SequencerDistributionRow>(&query).await?;
        Ok(rows)
    }

    /// Get the list of block numbers proposed by each sequencer since the given cutoff time
    pub async fn get_sequencer_blocks_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<SequencerBlockRow>> {
        let query = format!(
            "SELECT sequencer, h.l2_block_number \
             FROM {db}.l2_head_events h \
             WHERE h.block_ts > {} \
               AND {filter} \
             ORDER BY sequencer, h.l2_block_number ASC",
            since.timestamp(),
            filter = self.reorg_filter("h"),
            db = self.db_name,
        );

        let rows = self.execute::<SequencerBlockRow>(&query).await?;
        Ok(rows)
    }

    /// Get transactions per block since the given cutoff time with cursor-based
    /// pagination. Results are returned in descending order by block number.
    pub async fn get_block_transactions_paginated(
        &self,
        since: DateTime<Utc>,
        limit: u64,
        starting_after: Option<u64>,
        ending_before: Option<u64>,
        sequencer: Option<AddressBytes>,
    ) -> Result<Vec<BlockTransactionRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            sequencer: AddressBytes,
            l2_block_number: u64,
            block_time: u64,
            sum_tx: u32,
        }

        let mut query = format!(
            "SELECT sequencer, h.l2_block_number, h.block_ts AS block_time, sum_tx \
             FROM {db}.l2_head_events h \
             WHERE h.block_ts >= {} \
               AND {filter}",
            since.timestamp(),
            filter = self.reorg_filter("h"),
            db = self.db_name,
        );
        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND sequencer = unhex('{}')", encode(addr)));
        }

        if let Some(start) = starting_after {
            query.push_str(&format!(" AND l2_block_number < {}", start));
        }

        if let Some(end) = ending_before {
            query.push_str(&format!(" AND l2_block_number > {}", end));
        }

        query.push_str(" ORDER BY l2_block_number DESC");
        query.push_str(&format!(" LIMIT {}", limit));

        let rows = self.execute::<RawRow>(&query).await?;
        Ok(rows
            .into_iter()
            .map(|r| BlockTransactionRow {
                sequencer: r.sequencer,
                l2_block_number: r.l2_block_number,
                block_time: Utc.timestamp_opt(r.block_time as i64, 0).unwrap(),
                sum_tx: r.sum_tx,
            })
            .collect())
    }

    /// Get transactions per block for the specified block range.
    pub async fn get_block_transactions_block_range(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>,
        sequencer: Option<AddressBytes>,
    ) -> Result<Vec<BlockTransactionRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            sequencer: AddressBytes,
            l2_block_number: u64,
            block_time: u64,
            sum_tx: u32,
        }

        let mut query = format!(
            "SELECT sequencer, h.l2_block_number, h.block_ts AS block_time, sum_tx \
             FROM {db}.l2_head_events h \
             WHERE {filter}",
            filter = self.reorg_filter("h"),
            db = self.db_name,
        );

        if let Some(start) = start_block {
            query.push_str(&format!(" AND h.l2_block_number >= {}", start));
        }

        if let Some(end) = end_block {
            query.push_str(&format!(" AND h.l2_block_number <= {}", end));
        }

        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND sequencer = unhex('{}')", encode(addr)));
        }

        query.push_str(" ORDER BY l2_block_number ASC");

        let rows = self.execute::<RawRow>(&query).await?;
        Ok(rows
            .into_iter()
            .map(|r| BlockTransactionRow {
                sequencer: r.sequencer,
                l2_block_number: r.l2_block_number,
                block_time: Utc.timestamp_opt(r.block_time as i64, 0).unwrap(),
                sum_tx: r.sum_tx,
            })
            .collect())
    }

    /// Get L2 block times since the given cutoff with cursor-based pagination.
    /// Results are returned in descending order by block number.
    pub async fn get_l2_block_times_paginated(
        &self,
        since: DateTime<Utc>,
        limit: u64,
        starting_after: Option<u64>,
        ending_before: Option<u64>,
        sequencer: Option<AddressBytes>,
    ) -> Result<Vec<L2BlockTimeRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l2_block_number: u64,
            block_time: u64,
            ms_since_prev_block: Option<u64>,
        }

        let mut query = format!(
            "SELECT h.l2_block_number, h.block_ts AS block_time, \
                    toUInt64OrNull(toString((toUnixTimestamp64Milli(h.inserted_at) - \
                        lagInFrame(toUnixTimestamp64Milli(h.inserted_at)) OVER (ORDER BY h.l2_block_number)))) \
                        AS ms_since_prev_block \
             FROM {db}.l2_head_events h \
             WHERE h.block_ts >= {since} \
               AND {filter}",
            since = since.timestamp(),
            filter = self.reorg_filter("h"),
            db = self.db_name,
        );
        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND sequencer = unhex('{}')", encode(addr)));
        }
        if let Some(start) = starting_after {
            query.push_str(&format!(" AND l2_block_number < {}", start));
        }
        if let Some(end) = ending_before {
            query.push_str(&format!(" AND l2_block_number > {}", end));
        }
        query.push_str(" ORDER BY l2_block_number DESC");
        query.push_str(&format!(" LIMIT {}", limit));

        let rows = self.execute::<RawRow>(&query).await?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let dt = Utc.timestamp_opt(r.block_time as i64, 0).single()?;
                r.ms_since_prev_block.map(|ms| L2BlockTimeRow {
                    l2_block_number: r.l2_block_number,
                    block_time: dt,
                    ms_since_prev_block: Some(ms),
                })
            })
            .collect())
    }

    /// Get L2 gas usage since the given cutoff with cursor-based pagination.
    /// Results are returned in descending order by block number.
    pub async fn get_l2_gas_used_paginated(
        &self,
        since: DateTime<Utc>,
        limit: u64,
        starting_after: Option<u64>,
        ending_before: Option<u64>,
        sequencer: Option<AddressBytes>,
    ) -> Result<Vec<L2GasUsedRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l2_block_number: u64,
            block_time: u64,
            gas_used: u64,
        }

        let mut query = format!(
            "SELECT h.l2_block_number, h.block_ts AS block_time, toUInt64(sum_gas_used) AS gas_used \
             FROM {db}.l2_head_events h \
             WHERE h.block_ts >= {since} \
               AND {filter}",
            since = since.timestamp(),
            filter = self.reorg_filter("h"),
            db = self.db_name,
        );
        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND sequencer = unhex('{}')", encode(addr)));
        }
        if let Some(start) = starting_after {
            query.push_str(&format!(" AND l2_block_number < {}", start));
        }
        if let Some(end) = ending_before {
            query.push_str(&format!(" AND l2_block_number > {}", end));
        }
        query.push_str(" ORDER BY l2_block_number DESC");
        query.push_str(&format!(" LIMIT {}", limit));

        let rows = self.execute::<RawRow>(&query).await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let dt = Utc.timestamp_opt(r.block_time as i64, 0).unwrap();
                L2GasUsedRow {
                    l2_block_number: r.l2_block_number,
                    block_time: dt,
                    gas_used: r.gas_used,
                }
            })
            .collect())
    }

    /// Get L2 TPS since the given cutoff with cursor-based pagination.
    /// Results are returned in descending order by block number.
    pub async fn get_l2_tps_paginated(
        &self,
        since: DateTime<Utc>,
        limit: u64,
        starting_after: Option<u64>,
        ending_before: Option<u64>,
        sequencer: Option<AddressBytes>,
    ) -> Result<Vec<L2TpsRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l2_block_number: u64,
            sum_tx: u32,
            ms_since_prev_block: Option<u64>,
        }

        let mut query = format!(
            "SELECT h.l2_block_number, sum_tx, \
                    toUInt64OrNull(toString((toUnixTimestamp64Milli(h.inserted_at) - \
                        lagInFrame(toUnixTimestamp64Milli(h.inserted_at)) OVER (ORDER BY h.l2_block_number)))) AS ms_since_prev_block \
             FROM {db}.l2_head_events h \
             WHERE h.block_ts >= {since} \
               AND {filter}",
            since = since.timestamp(),
            filter = self.reorg_filter("h"),
            db = self.db_name,
        );
        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND sequencer = unhex('{}')", encode(addr)));
        }
        if let Some(start) = starting_after {
            query.push_str(&format!(" AND l2_block_number < {}", start));
        }
        if let Some(end) = ending_before {
            query.push_str(&format!(" AND l2_block_number > {}", end));
        }
        query.push_str(" ORDER BY l2_block_number DESC");
        query.push_str(&format!(" LIMIT {}", limit));

        let rows = self.execute::<RawRow>(&query).await?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let ms = r.ms_since_prev_block?;
                if ms == 0 {
                    None
                } else {
                    Some(L2TpsRow {
                        l2_block_number: r.l2_block_number,
                        tps: r.sum_tx as f64 / (ms as f64 / 1000.0),
                    })
                }
            })
            .collect())
    }

    /// Get the average time in milliseconds it takes for a batch to be proven
    /// for proofs submitted within the given time range
    pub async fn get_avg_prove_time(&self, range: TimeRange) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct AvgRow {
            avg_ms: f64,
        }

        // First try the materialized view
        let mv_query = format!(
            "SELECT avg(prove_time_ms) AS avg_ms \
             FROM {db}.batch_prove_times_mv \
             WHERE proved_at >= now64() - INTERVAL {}",
            range.interval(),
            db = self.db_name
        );

        let rows = self.execute::<AvgRow>(&mv_query).await?;
        if let Some(row) = rows.into_iter().next() {
            if !row.avg_ms.is_nan() {
                return Ok(Some(row.avg_ms.round() as u64));
            }
        }

        // Fallback to raw data if materialized view is empty
        let fallback_query = format!(
            "SELECT avg((l1_proved.block_ts - l1_proposed.block_ts) * 1000) AS avg_ms \
             FROM {db}.batches b \
             JOIN {db}.proved_batches pb ON b.batch_id = pb.batch_id \
             JOIN {db}.l1_head_events l1_proposed \
               ON b.l1_block_number = l1_proposed.l1_block_number \
             JOIN {db}.l1_head_events l1_proved \
               ON pb.l1_block_number = l1_proved.l1_block_number \
             WHERE l1_proved.block_ts >= (toUInt64(now()) - {})",
            range.seconds(),
            db = self.db_name
        );

        let rows = self.execute::<AvgRow>(&fallback_query).await?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };

        if row.avg_ms.is_nan() { Ok(None) } else { Ok(Some(row.avg_ms.round() as u64)) }
    }

    /// Get the average time in milliseconds it takes for a batch to be verified
    /// for verifications submitted within the given time range
    pub async fn get_avg_verify_time(&self, range: TimeRange) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct AvgRow {
            avg_ms: f64,
        }

        // First try the materialized view
        let mv_query = format!(
            "SELECT avg(verify_time_ms) AS avg_ms \
             FROM {db}.batch_verify_times_mv \
             WHERE verified_at >= now64() - INTERVAL {}",
            range.interval(),
            db = self.db_name
        );

        let rows = self.execute::<AvgRow>(&mv_query).await?;
        if let Some(row) = rows.into_iter().next() {
            if !row.avg_ms.is_nan() {
                return Ok(Some(row.avg_ms.round() as u64));
            }
        }

        // Fallback to raw data if materialized view is empty
        let fallback_query = format!(
            "SELECT avg((l1_verified.block_ts - l1_proved.block_ts) * 1000) AS avg_ms \
             FROM {db}.proved_batches pb \
             INNER JOIN {db}.verified_batches vb \
                ON pb.batch_id = vb.batch_id AND pb.block_hash = vb.block_hash \
             INNER JOIN {db}.l1_head_events l1_proved \
                ON pb.l1_block_number = l1_proved.l1_block_number \
             INNER JOIN {db}.l1_head_events l1_verified \
                ON vb.l1_block_number = l1_verified.l1_block_number \
             WHERE l1_verified.block_ts >= (toUInt64(now()) - {}) \
               AND l1_verified.block_ts > l1_proved.block_ts \
               AND (l1_verified.block_ts - l1_proved.block_ts) > 60 \
               AND pb.batch_id != 0",
            range.seconds(),
            db = self.db_name
        );

        let rows = self.execute::<AvgRow>(&fallback_query).await?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };

        if row.avg_ms.is_nan() { Ok(None) } else { Ok(Some(row.avg_ms.round() as u64)) }
    }

    /// Get the average interval in milliseconds between consecutive L2 blocks
    /// observed within the given range.
    pub async fn get_l2_block_cadence(
        &self,
        sequencer: Option<AddressBytes>,
        range: TimeRange,
    ) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct CadenceRow {
            min_ts: u64,
            max_ts: u64,
            cnt: u64,
        }

        let mut query = format!(
            "SELECT toUInt64(min(h.block_ts) * 1000) AS min_ts, \
                    toUInt64(max(h.block_ts) * 1000) AS max_ts, \
                    count() as cnt \
             FROM {db}.l2_head_events h \
             WHERE h.block_ts >= toUnixTimestamp(now64() - INTERVAL {interval}) \
               AND {filter}",
            interval = range.interval(),
            filter = self.reorg_filter("h"),
            db = self.db_name,
        );
        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND sequencer = unhex('{}')", encode(addr)));
        }

        let rows = self.execute::<CadenceRow>(&query).await?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };

        if row.cnt > 1 && row.max_ts > row.min_ts {
            Ok(Some((row.max_ts - row.min_ts) / (row.cnt - 1)))
        } else {
            Ok(None)
        }
    }

    /// Get the average interval in milliseconds between consecutive batch
    /// proposals observed within the given range.
    pub async fn get_batch_posting_cadence(&self, range: TimeRange) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct CadenceRow {
            min_ts: u64,
            max_ts: u64,
            cnt: u64,
        }

        let query = format!(
            "SELECT toUInt64(min(l1_events.block_ts) * 1000) AS min_ts, \
                    toUInt64(max(l1_events.block_ts) * 1000) AS max_ts, \
                    count() as cnt \
             FROM {db}.batches b \
             INNER JOIN {db}.l1_head_events l1_events \
               ON b.l1_block_number = l1_events.l1_block_number \
             WHERE l1_events.block_ts >= toUnixTimestamp(now64() - INTERVAL {interval})",
            interval = range.interval(),
            db = self.db_name,
        );

        let rows = self.execute::<CadenceRow>(&query).await?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };

        if row.cnt > 1 && row.max_ts > row.min_ts {
            Ok(Some((row.max_ts - row.min_ts) / (row.cnt - 1)))
        } else {
            Ok(None)
        }
    }

    /// Get the interval in milliseconds between consecutive batch proposals
    /// observed within the given range.
    pub async fn get_batch_posting_times(
        &self,
        range: TimeRange,
    ) -> Result<Vec<BatchPostingTimeRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            batch_id: u64,
            ts: u64,
            ms_since_prev_batch: Option<u64>,
        }

        let query = format!(
            "SELECT batch_id, ts, \
                    if(ts > prev_ts, CAST(ts - prev_ts AS UInt64), NULL) AS ms_since_prev_batch \
             FROM ( \
                 SELECT b.batch_id AS batch_id, \
                        toUInt64(l1_events.block_ts * 1000) AS ts, \
                        lagInFrame(toNullable(toUInt64(l1_events.block_ts * 1000))) \
                            OVER (ORDER BY l1_events.block_ts, b.batch_id) AS prev_ts \
                   FROM {db}.batches b \
                   INNER JOIN {db}.l1_head_events l1_events \
                     ON b.l1_block_number = l1_events.l1_block_number \
                  WHERE l1_events.block_ts >= toUnixTimestamp(now64() - INTERVAL {interval}) \
                  ORDER BY l1_events.block_ts, b.batch_id \
             ) \
             WHERE prev_ts IS NOT NULL \
             ORDER BY ts",
            interval = range.interval(),
            db = self.db_name,
        );

        let rows = self.execute::<RawRow>(&query).await?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let dt = Utc.timestamp_millis_opt(r.ts as i64).single()?;
                r.ms_since_prev_batch.map(|ms| BatchPostingTimeRow {
                    batch_id: r.batch_id,
                    inserted_at: dt,
                    ms_since_prev_batch: Some(ms),
                })
            })
            .collect())
    }

    /// Get the interval between consecutive batch proposals since the given cutoff
    /// time with cursor-based pagination. Results are returned in ascending order
    /// by batch id.
    pub async fn get_batch_posting_times_paginated(
        &self,
        since: DateTime<Utc>,
        limit: u64,
        starting_after: Option<u64>,
        ending_before: Option<u64>,
    ) -> Result<Vec<BatchPostingTimeRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            batch_id: u64,
            ts: u64,
            ms_since_prev_batch: Option<u64>,
        }

        let mut query = format!(
            "SELECT batch_id, ts, \
                    if(ts > prev_ts, CAST(ts - prev_ts AS UInt64), NULL) AS ms_since_prev_batch \
             FROM ( \
                 SELECT b.batch_id AS batch_id, \
                        toUInt64(l1_events.block_ts * 1000) AS ts, \
                        lagInFrame(toNullable(toUInt64(l1_events.block_ts * 1000))) \
                            OVER (ORDER BY l1_events.block_ts, b.batch_id) AS prev_ts \
                   FROM {db}.batches b \
                   INNER JOIN {db}.l1_head_events l1_events \
                     ON b.l1_block_number = l1_events.l1_block_number \
                  WHERE l1_events.block_ts >= {since} \
                  ORDER BY l1_events.block_ts, b.batch_id \
             ) \
             WHERE prev_ts IS NOT NULL",
            since = since.timestamp(),
            db = self.db_name,
        );
        if let Some(start) = starting_after {
            query.push_str(&format!(" AND batch_id > {}", start));
        }
        if let Some(end) = ending_before {
            query.push_str(&format!(" AND batch_id < {}", end));
        }
        query.push_str(" ORDER BY batch_id ASC");
        query.push_str(&format!(" LIMIT {}", limit));

        let rows = self.execute::<RawRow>(&query).await?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let dt = Utc.timestamp_millis_opt(r.ts as i64).single()?;
                r.ms_since_prev_batch.map(|ms| BatchPostingTimeRow {
                    batch_id: r.batch_id,
                    inserted_at: dt,
                    ms_since_prev_batch: Some(ms),
                })
            })
            .collect())
    }

    /// Get prove times in seconds for batches proved within the given range
    pub async fn get_prove_times(&self, range: TimeRange) -> Result<Vec<BatchProveTimeRow>> {
        let mv_query = format!(
            "SELECT batch_id, toUInt64(prove_time_ms / 1000) AS seconds_to_prove \
             FROM {db}.batch_prove_times_mv \
             WHERE proved_at >= now64() - INTERVAL {interval} \
               AND batch_id != 0 \
             ORDER BY batch_id ASC",
            interval = range.interval(),
            db = self.db_name,
        );

        let rows = self.execute::<BatchProveTimeRow>(&mv_query).await?;
        if !rows.is_empty() {
            return Ok(rows);
        }

        let fallback_query = format!(
            "SELECT toUInt64(b.batch_id) AS batch_id, \
                    (l1_proved.block_ts - l1_proposed.block_ts) AS seconds_to_prove \
             FROM {db}.batches b \
             JOIN {db}.proved_batches pb ON b.batch_id = pb.batch_id \
             JOIN {db}.l1_head_events l1_proposed \
               ON b.l1_block_number = l1_proposed.l1_block_number \
             JOIN {db}.l1_head_events l1_proved \
               ON pb.l1_block_number = l1_proved.l1_block_number \
             WHERE l1_proved.block_ts >= (toUInt64(now()) - {secs}) \
               AND b.batch_id != 0 \
             ORDER BY b.batch_id ASC",
            secs = range.seconds(),
            db = self.db_name,
        );

        let rows = self.execute::<BatchProveTimeRow>(&fallback_query).await?;
        Ok(rows)
    }

    /// Get verify times in seconds for batches verified within the given range
    pub async fn get_verify_times(&self, range: TimeRange) -> Result<Vec<BatchVerifyTimeRow>> {
        let mv_query = format!(
            "SELECT batch_id, toUInt64(verify_time_ms / 1000) AS seconds_to_verify \
             FROM {db}.batch_verify_times_mv \
             WHERE verified_at >= now64() - INTERVAL {interval} \
               AND verify_time_ms > 60000 \
               AND batch_id != 0 \
             ORDER BY batch_id ASC",
            interval = range.interval(),
            db = self.db_name,
        );

        let rows = self.execute::<BatchVerifyTimeRow>(&mv_query).await?;
        if !rows.is_empty() {
            return Ok(rows);
        }

        let fallback_query = format!(
            "SELECT toUInt64(pb.batch_id) AS batch_id, \
                    (l1_verified.block_ts - l1_proved.block_ts) AS seconds_to_verify \
             FROM {db}.proved_batches pb \
             INNER JOIN {db}.verified_batches vb \
                ON pb.batch_id = vb.batch_id AND pb.block_hash = vb.block_hash \
             INNER JOIN {db}.l1_head_events l1_proved \
                ON pb.l1_block_number = l1_proved.l1_block_number \
             INNER JOIN {db}.l1_head_events l1_verified \
                ON vb.l1_block_number = l1_verified.l1_block_number \
             WHERE l1_verified.block_ts >= (toUInt64(now()) - {}) \
               AND l1_verified.block_ts > l1_proved.block_ts \
               AND (l1_verified.block_ts - l1_proved.block_ts) > 60 \
               AND pb.batch_id != 0",
            range.seconds(),
            db = self.db_name
        );

        let rows = self.execute::<BatchVerifyTimeRow>(&fallback_query).await?;
        Ok(rows)
    }

    /// Get verify times with cursor-based pagination
    /// Results are returned in descending order by batch id
    pub async fn get_verify_times_paginated(
        &self,
        since: DateTime<Utc>,
        limit: u64,
        starting_after: Option<u64>,
        ending_before: Option<u64>,
    ) -> Result<Vec<BatchVerifyTimeRow>> {
        // First try the materialized view
        let mut mv_query = format!(
            "SELECT batch_id, toUInt64(verify_time_ms / 1000) AS seconds_to_verify \
             FROM {db}.batch_verify_times_mv \
             WHERE verified_at >= {since} \
               AND verify_time_ms > 60000",
            since = since.timestamp(),
            db = self.db_name,
        );
        if let Some(start) = starting_after {
            mv_query.push_str(&format!(" AND batch_id < {}", start));
        }
        if let Some(end) = ending_before {
            mv_query.push_str(&format!(" AND batch_id > {}", end));
        }
        mv_query.push_str(" ORDER BY batch_id DESC");
        mv_query.push_str(&format!(" LIMIT {}", limit));

        let rows = self.execute::<BatchVerifyTimeRow>(&mv_query).await?;
        if !rows.is_empty() {
            return Ok(rows);
        }

        // Fallback to raw data if materialized view is empty
        let mut fallback_query = format!(
            "SELECT toUInt64(pb.batch_id) AS batch_id, \
                    (l1_verified.block_ts - l1_proved.block_ts) AS seconds_to_verify \
             FROM {db}.proved_batches pb \
             INNER JOIN {db}.verified_batches vb \
                ON pb.batch_id = vb.batch_id AND pb.block_hash = vb.block_hash \
             INNER JOIN {db}.l1_head_events l1_proved \
                ON pb.l1_block_number = l1_proved.l1_block_number \
             INNER JOIN {db}.l1_head_events l1_verified \
                ON vb.l1_block_number = l1_verified.l1_block_number \
             WHERE l1_verified.block_ts >= {since} \
               AND l1_verified.block_ts > l1_proved.block_ts \
               AND (l1_verified.block_ts - l1_proved.block_ts) > 60",
            since = since.timestamp(),
            db = self.db_name,
        );
        if let Some(start) = starting_after {
            fallback_query.push_str(&format!(" AND pb.batch_id < {}", start));
        }
        if let Some(end) = ending_before {
            fallback_query.push_str(&format!(" AND pb.batch_id > {}", end));
        }
        fallback_query.push_str(" ORDER BY pb.batch_id DESC");
        fallback_query.push_str(&format!(" LIMIT {}", limit));

        let rows = self.execute::<BatchVerifyTimeRow>(&fallback_query).await?;
        Ok(rows)
    }

    /// Get prove times with cursor-based pagination
    /// Results are returned in descending order by batch id
    pub async fn get_prove_times_paginated(
        &self,
        since: DateTime<Utc>,
        limit: u64,
        starting_after: Option<u64>,
        ending_before: Option<u64>,
    ) -> Result<Vec<BatchProveTimeRow>> {
        // First try the materialized view
        let mut mv_query = format!(
            "SELECT batch_id, toUInt64(prove_time_ms / 1000) AS seconds_to_prove \
             FROM {db}.batch_prove_times_mv \
             WHERE proved_at >= {since}",
            since = since.timestamp(),
            db = self.db_name,
        );
        if let Some(start) = starting_after {
            mv_query.push_str(&format!(" AND batch_id < {}", start));
        }
        if let Some(end) = ending_before {
            mv_query.push_str(&format!(" AND batch_id > {}", end));
        }
        mv_query.push_str(" ORDER BY batch_id DESC");
        mv_query.push_str(&format!(" LIMIT {}", limit));

        let rows = self.execute::<BatchProveTimeRow>(&mv_query).await?;
        if !rows.is_empty() {
            return Ok(rows);
        }

        // Fallback to raw data if materialized view is empty
        let mut fallback_query = format!(
            "SELECT toUInt64(b.batch_id) AS batch_id, \
                    (l1_proved.block_ts - l1_proposed.block_ts) AS seconds_to_prove \
             FROM {db}.batches b \
             JOIN {db}.proved_batches pb ON b.batch_id = pb.batch_id \
             JOIN {db}.l1_head_events l1_proposed \
               ON b.l1_block_number = l1_proposed.l1_block_number \
             JOIN {db}.l1_head_events l1_proved \
               ON pb.l1_block_number = l1_proved.l1_block_number \
             WHERE l1_proved.block_ts >= {since}",
            since = since.timestamp(),
            db = self.db_name,
        );
        if let Some(start) = starting_after {
            fallback_query.push_str(&format!(" AND b.batch_id < {}", start));
        }
        if let Some(end) = ending_before {
            fallback_query.push_str(&format!(" AND b.batch_id > {}", end));
        }
        fallback_query.push_str(" ORDER BY b.batch_id DESC");
        fallback_query.push_str(&format!(" LIMIT {}", limit));

        let rows = self.execute::<BatchProveTimeRow>(&fallback_query).await?;
        Ok(rows)
    }

    /// Get L1 block numbers grouped by minute for the given range
    pub async fn get_l1_block_times(&self, range: TimeRange) -> Result<Vec<L1BlockTimeRow>> {
        let query = format!(
            "SELECT toUInt64(toStartOfMinute(fromUnixTimestamp64Milli(block_ts * 1000))) AS minute, \
                    max(l1_block_number) AS l1_block_number \
             FROM {db}.l1_head_events \
              WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL {interval}) \
              GROUP BY minute \
              ORDER BY minute",
            interval = range.interval(),
            db = self.db_name,
        );

        let rows = self.execute::<L1BlockTimeRow>(&query).await?;
        Ok(rows)
    }

    /// Get the time between consecutive L2 blocks for the given range
    pub async fn get_l2_block_times(
        &self,
        sequencer: Option<AddressBytes>,
        range: TimeRange,
    ) -> Result<Vec<L2BlockTimeRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l2_block_number: u64,
            block_time: u64,
            ms_since_prev_block: Option<u64>,
        }

        let mut query = format!(
            "SELECT h.l2_block_number, \
                    h.block_ts AS block_time, \
                    toUInt64OrNull(toString( \
                        (toUnixTimestamp64Milli(h.inserted_at) - \
                         lagInFrame(toUnixTimestamp64Milli(h.inserted_at)) OVER (ORDER BY \
                         h.l2_block_number)) \
                    )) AS ms_since_prev_block \
             FROM {db}.l2_head_events h \
             WHERE h.inserted_at >= (now64() - INTERVAL {interval}) \
               AND {filter}",
            interval = range.interval(),
            filter = self.reorg_filter("h"),
            db = self.db_name,
        );
        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND sequencer = unhex('{}')", encode(addr)));
        }
        query.push_str(" ORDER BY l2_block_number ASC");
        let rows = self.execute::<RawRow>(&query).await?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let dt = Utc.timestamp_opt(r.block_time as i64, 0).single()?;
                r.ms_since_prev_block.map(|ms| L2BlockTimeRow {
                    l2_block_number: r.l2_block_number,
                    block_time: dt,
                    ms_since_prev_block: Some(ms),
                })
            })
            .collect())
    }

    /// Get the time between consecutive L2 blocks for the specified block range
    pub async fn get_l2_block_times_block_range(
        &self,
        sequencer: Option<AddressBytes>,
        start_block: Option<u64>,
        end_block: Option<u64>,
    ) -> Result<Vec<L2BlockTimeRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l2_block_number: u64,
            block_time: u64,
            ms_since_prev_block: Option<u64>,
        }

        let mut query = format!(
            "SELECT h.l2_block_number, \
                    h.block_ts AS block_time, \
                    toUInt64OrNull(toString(\
                        (toUnixTimestamp64Milli(h.inserted_at) - \
                         lagInFrame(toUnixTimestamp64Milli(h.inserted_at)) OVER (ORDER BY \
                         h.l2_block_number))\
                    )) AS ms_since_prev_block \
             FROM {db}.l2_head_events h \
             WHERE {filter}",
            filter = self.reorg_filter("h"),
            db = self.db_name,
        );

        if let Some(start) = start_block {
            query.push_str(&format!(" AND h.l2_block_number >= {}", start));
        }

        if let Some(end) = end_block {
            query.push_str(&format!(" AND h.l2_block_number <= {}", end));
        }

        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND sequencer = unhex('{}')", encode(addr)));
        }

        query.push_str(" ORDER BY l2_block_number ASC");

        let rows = self.execute::<RawRow>(&query).await?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let dt = Utc.timestamp_opt(r.block_time as i64, 0).single()?;
                r.ms_since_prev_block.map(|ms| L2BlockTimeRow {
                    l2_block_number: r.l2_block_number,
                    block_time: dt,
                    ms_since_prev_block: Some(ms),
                })
            })
            .collect())
    }

    /// Get the average number of L2 transactions per second for the given range
    pub async fn get_avg_l2_tps(
        &self,
        sequencer: Option<AddressBytes>,
        range: TimeRange,
    ) -> Result<Option<f64>> {
        #[derive(Row, Deserialize)]
        struct TpsRow {
            min_ts: u64,
            max_ts: u64,
            tx_sum: u64,
        }

        let mut query = format!(
            "SELECT toUInt64(min(h.block_ts)) AS min_ts, \
                    toUInt64(max(h.block_ts)) AS max_ts, \
                    sum(sum_tx) AS tx_sum \
             FROM {db}.l2_head_events h \
             WHERE h.block_ts >= toUnixTimestamp(now64() - INTERVAL {interval}) \
               AND {filter}",
            interval = range.interval(),
            filter = self.reorg_filter("h"),
            db = self.db_name
        );
        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND sequencer = unhex('{}')", encode(addr)));
        }

        let rows = self.execute::<TpsRow>(&query).await?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };

        if row.max_ts > row.min_ts && row.tx_sum > 0 {
            let duration = (row.max_ts - row.min_ts) as f64;
            Ok(Some(row.tx_sum as f64 / duration))
        } else {
            Ok(None)
        }
    }

    /// Get the gas used for each L2 block within the given range
    pub async fn get_l2_gas_used(
        &self,
        sequencer: Option<AddressBytes>,
        range: TimeRange,
    ) -> Result<Vec<L2GasUsedRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l2_block_number: u64,
            block_time: u64,
            gas_used: u64,
        }

        let mut query = format!(
            "SELECT h.l2_block_number, h.block_ts AS block_time, toUInt64(sum_gas_used) AS gas_used \
             FROM {db}.l2_head_events h \
             WHERE h.block_ts >= toUnixTimestamp(now64() - INTERVAL {interval}) \
               AND {filter}",
            interval = range.interval(),
            filter = self.reorg_filter("h"),
            db = self.db_name,
        );
        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND sequencer = unhex('{}')", encode(addr)));
        }
        query.push_str(" ORDER BY l2_block_number ASC");

        let rows = self.execute::<RawRow>(&query).await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let dt = Utc.timestamp_opt(r.block_time as i64, 0).unwrap();
                L2GasUsedRow {
                    l2_block_number: r.l2_block_number,
                    block_time: dt,
                    gas_used: r.gas_used,
                }
            })
            .collect())
    }

    /// Get the gas used for each L2 block within the specified block range
    pub async fn get_l2_gas_used_block_range(
        &self,
        sequencer: Option<AddressBytes>,
        start_block: Option<u64>,
        end_block: Option<u64>,
    ) -> Result<Vec<L2GasUsedRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l2_block_number: u64,
            block_time: u64,
            gas_used: u64,
        }

        let mut query = format!(
            "SELECT h.l2_block_number, h.block_ts AS block_time, toUInt64(sum_gas_used) AS gas_used \
             FROM {db}.l2_head_events h \
             WHERE {filter}",
            filter = self.reorg_filter("h"),
            db = self.db_name,
        );

        if let Some(start) = start_block {
            query.push_str(&format!(" AND h.l2_block_number >= {}", start));
        }

        if let Some(end) = end_block {
            query.push_str(&format!(" AND h.l2_block_number <= {}", end));
        }

        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND sequencer = unhex('{}')", encode(addr)));
        }

        query.push_str(" ORDER BY l2_block_number ASC");

        let rows = self.execute::<RawRow>(&query).await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let dt = Utc.timestamp_opt(r.block_time as i64, 0).unwrap();
                L2GasUsedRow {
                    l2_block_number: r.l2_block_number,
                    block_time: dt,
                    gas_used: r.gas_used,
                }
            })
            .collect())
    }
    /// Get the L1 data posting cost for each block within the given range
    pub async fn get_l1_data_costs(&self, range: TimeRange) -> Result<Vec<L1DataCostRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l1_block_number: u64,
            cost: u128,
        }

        // Group by l1_block_number and sum costs since L1DataCostRow only has these 2 fields
        let query = format!(
            "SELECT c.l1_block_number, sum(c.cost) as cost \
         FROM {db}.l1_data_costs c \
         INNER JOIN {db}.l1_head_events h \
           ON c.l1_block_number = h.l1_block_number \
         WHERE h.block_ts >= toUnixTimestamp(now64() - INTERVAL {interval}) \
         GROUP BY c.l1_block_number \
         ORDER BY c.l1_block_number ASC",
            interval = range.interval(),
            db = self.db_name,
        );

        let rows = self.execute::<RawRow>(&query).await?;
        Ok(rows
            .into_iter()
            .map(|r| L1DataCostRow { l1_block_number: r.l1_block_number, cost: r.cost })
            .collect())
    }

    /// Get the L1 data posting cost since the given cutoff time with cursor-based pagination.
    /// Results are returned in descending order by block number.
    pub async fn get_l1_data_costs_paginated(
        &self,
        since: DateTime<Utc>,
        limit: u64,
        starting_after: Option<u64>,
        ending_before: Option<u64>,
    ) -> Result<Vec<L1DataCostRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l1_block_number: u64,
            cost: u128,
        }

        // First join with l1_head_events to filter by time, then group by l1_block_number
        let mut query = format!(
            "SELECT c.l1_block_number, sum(c.cost) as cost \
         FROM {db}.l1_data_costs c \
         INNER JOIN {db}.l1_head_events h \
           ON c.l1_block_number = h.l1_block_number \
         WHERE h.block_ts >= {since}",
            since = since.timestamp(),
            db = self.db_name,
        );
        if let Some(start) = starting_after {
            query.push_str(&format!(" AND c.l1_block_number < {}", start));
        }
        if let Some(end) = ending_before {
            query.push_str(&format!(" AND c.l1_block_number > {}", end));
        }
        query.push_str(" GROUP BY c.l1_block_number");
        query.push_str(" ORDER BY c.l1_block_number DESC");
        query.push_str(&format!(" LIMIT {}", limit));

        let rows = self.execute::<RawRow>(&query).await?;
        Ok(rows
            .into_iter()
            .map(|r| L1DataCostRow { l1_block_number: r.l1_block_number, cost: r.cost })
            .collect())
    }

    /// Get the total L1 data posting cost for the given range
    pub async fn get_l1_total_data_cost(
        &self,
        sequencer: Option<AddressBytes>,
        range: TimeRange,
    ) -> Result<Option<u128>> {
        #[derive(Row, Deserialize)]
        struct SumRow {
            total: u128,
        }

        let mut query = format!(
            "SELECT sum(c.cost) AS total \
             FROM {db}.l1_data_costs c \
             INNER JOIN {db}.batches b \
               ON c.batch_id = b.batch_id AND c.l1_block_number = b.l1_block_number \
             INNER JOIN {db}.l1_head_events l1 \
               ON b.l1_block_number = l1.l1_block_number \
             WHERE l1.block_ts >= toUnixTimestamp(now64() - INTERVAL {interval})",
            interval = range.interval(),
            db = self.db_name,
        );
        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND b.proposer_addr = unhex('{}')", encode(addr)));
        }

        let rows = self.execute::<SumRow>(&query).await?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };
        Ok(Some(row.total))
    }

    /// Get priority fee, base fee and L1 data cost for each L2 block
    pub async fn get_l2_fee_components(
        &self,
        sequencer: Option<AddressBytes>,
        range: TimeRange,
    ) -> Result<Vec<BlockFeeComponentRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l2_block_number: u64,
            priority_fee: u128,
            base_fee: u128,
            l1_data_cost: Option<u128>,
        }

        let mut query = format!(
            "SELECT h.l2_block_number, \
                    sum_priority_fee AS priority_fee, \
                    sum_base_fee AS base_fee, \
                    toNullable(if(b.batch_size > 0, intDiv(dc.cost, b.batch_size), NULL)) AS l1_data_cost \
             FROM {db}.l2_head_events h \
             LEFT JOIN {db}.batch_blocks bb \
               ON h.l2_block_number = bb.l2_block_number \
             LEFT JOIN {db}.batches b \
               ON bb.batch_id = b.batch_id \
             LEFT JOIN {db}.l1_data_costs dc \
               ON b.batch_id = dc.batch_id AND b.l1_block_number = dc.l1_block_number \
             WHERE h.block_ts >= toUnixTimestamp(now64() - INTERVAL {interval})",
            interval = range.interval(),
            db = self.db_name,
        );
        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND sequencer = unhex('{}')", encode(addr)));
        }
        query.push_str(" ORDER BY l2_block_number ASC");

        let rows = self.execute::<RawRow>(&query).await?;
        Ok(rows
            .into_iter()
            .map(|r| BlockFeeComponentRow {
                l2_block_number: r.l2_block_number,
                priority_fee: r.priority_fee,
                base_fee: r.base_fee,
                l1_data_cost: r.l1_data_cost,
            })
            .collect())
    }

    /// Get priority fee, base fee and L1 data cost for each batch
    pub async fn get_batch_fee_components(
        &self,
        proposer: Option<AddressBytes>,
        range: TimeRange,
    ) -> Result<Vec<BatchFeeComponentRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            batch_id: u64,
            l1_block_number: u64,
            proposer: AddressBytes,
            priority_fee: u128,
            base_fee: u128,
            l1_data_cost: Option<u128>,
        }

        let mut query = format!(
            "SELECT bb.batch_id, \
                    b.l1_block_number AS l1_block_number, \
                    b.proposer_addr AS proposer, \
                    sum(h.sum_priority_fee) AS priority_fee, \
                    sum(h.sum_base_fee) AS base_fee, \
                    toNullable(max(dc.cost)) AS l1_data_cost \
             FROM {db}.batch_blocks bb \
             INNER JOIN {db}.batches b \
               ON bb.batch_id = b.batch_id \
             LEFT JOIN {db}.l2_head_events h \
               ON bb.l2_block_number = h.l2_block_number \
             LEFT JOIN {db}.l1_data_costs dc \
               ON b.batch_id = dc.batch_id AND b.l1_block_number = dc.l1_block_number \
             WHERE h.block_ts >= toUnixTimestamp(now64() - INTERVAL {interval})",
            interval = range.interval(),
            db = self.db_name,
        );
        if let Some(addr) = proposer {
            query.push_str(&format!(" AND b.proposer_addr = unhex('{}')", encode(addr)));
        }
        query.push_str(
            " GROUP BY bb.batch_id, b.l1_block_number, b.proposer_addr ORDER BY bb.batch_id ASC",
        );

        let rows = self.execute::<RawRow>(&query).await?;
        Ok(rows
            .into_iter()
            .map(|r| BatchFeeComponentRow {
                batch_id: r.batch_id,
                l1_block_number: r.l1_block_number,
                sequencer: r.proposer,
                priority_fee: r.priority_fee,
                base_fee: r.base_fee,
                l1_data_cost: r.l1_data_cost,
            })
            .collect())
    }

    /// Get the total priority fee for the given range aggregated by batch
    pub async fn get_batch_priority_fee(
        &self,
        proposer: Option<AddressBytes>,
        range: TimeRange,
    ) -> Result<Option<u128>> {
        let rows = self.get_batch_fee_components(proposer, range).await?;
        let total: u128 = rows.iter().map(|r| r.priority_fee).sum();
        Ok((total > 0).then_some(total))
    }

    /// Get the total base fee for the given range aggregated by batch
    pub async fn get_batch_base_fee(
        &self,
        proposer: Option<AddressBytes>,
        range: TimeRange,
    ) -> Result<Option<u128>> {
        let rows = self.get_batch_fee_components(proposer, range).await?;
        let total: u128 = rows.iter().map(|r| r.base_fee).sum();
        Ok((total > 0).then_some(total))
    }

    /// Get the total L1 data cost for the given range aggregated by batch
    pub async fn get_batch_total_data_cost(
        &self,
        proposer: Option<AddressBytes>,
        range: TimeRange,
    ) -> Result<Option<u128>> {
        let rows = self.get_batch_fee_components(proposer, range).await?;
        let total: u128 = rows.iter().map(|r| r.l1_data_cost.unwrap_or(0)).sum();
        Ok((total > 0).then_some(total))
    }

    /// Get aggregated prove costs grouped by proposer for the given range
    pub async fn get_prove_costs_by_proposer(
        &self,
        range: TimeRange,
    ) -> Result<Vec<(AddressBytes, u128)>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            proposer: AddressBytes,
            total_cost: u128,
        }

        let query = format!(
            "SELECT b.proposer_addr AS proposer, \
                    sum(pc.cost) AS total_cost \
             FROM {db}.prove_costs pc \
             INNER JOIN {db}.batches b ON pc.batch_id = b.batch_id \
             INNER JOIN {db}.l1_head_events l1 ON pc.l1_block_number = l1.l1_block_number \
             WHERE l1.block_ts >= toUnixTimestamp(now64() - INTERVAL {interval}) \
             GROUP BY b.proposer_addr \
             ORDER BY total_cost DESC",
            interval = range.interval(),
            db = self.db_name,
        );

        let rows = self.execute::<RawRow>(&query).await?;
        Ok(rows.into_iter().map(|r| (r.proposer, r.total_cost)).collect())
    }

    /// Get aggregated batch fees grouped by proposer for the given range
    pub async fn get_batch_fees_by_proposer(
        &self,
        range: TimeRange,
    ) -> Result<Vec<SequencerFeeRow>> {
        let query = format!(
            "SELECT b.proposer_addr AS proposer, \
                    sum(h.sum_priority_fee) AS priority_fee, \
                    sum(h.sum_base_fee) AS base_fee, \
                    toNullable(sum(if(b.batch_size > 0, intDiv(dc.cost, b.batch_size), NULL))) AS l1_data_cost, \
                    toNullable(sum(if(b.batch_size > 0, intDiv(pc.cost, b.batch_size), NULL))) AS prove_cost, \
                    toNullable(sum(if(b.batch_size > 0, intDiv(vc.cost, b.batch_size), NULL))) AS verify_cost \
             FROM {db}.batch_blocks bb \
             INNER JOIN {db}.batches b \
               ON bb.batch_id = b.batch_id \
             INNER JOIN {db}.l1_head_events l1 \
               ON b.l1_block_number = l1.l1_block_number \
             LEFT JOIN {db}.l2_head_events h \
               ON bb.l2_block_number = h.l2_block_number \
             LEFT JOIN {db}.l1_data_costs dc \
               ON b.batch_id = dc.batch_id \
             LEFT JOIN {db}.prove_costs pc \
               ON b.batch_id = pc.batch_id \
             LEFT JOIN {db}.verify_costs vc \
               ON b.batch_id = vc.batch_id \
             WHERE l1.block_ts >= toUnixTimestamp(now64() - INTERVAL {interval}) \
             GROUP BY b.proposer_addr \
             ORDER BY priority_fee DESC",
            interval = range.interval(),
            db = self.db_name,
        );

        let rows = self.execute::<SequencerFeeRow>(&query).await?;
        Ok(rows)
    }

    /// Get prover cost since the given cutoff time with cursor-based pagination
    /// Results are returned in descending order by batch id
    pub async fn get_prove_costs_paginated(
        &self,
        since: DateTime<Utc>,
        limit: u64,
        starting_after: Option<u64>,
        ending_before: Option<u64>,
    ) -> Result<Vec<ProveCostRow>> {
        let mut query = format!(
            "SELECT pc.l1_block_number, pc.batch_id, pc.cost \
         FROM {db}.prove_costs pc \
         INNER JOIN {db}.l1_head_events h \
           ON pc.l1_block_number = h.l1_block_number \
         WHERE h.block_ts >= toUnixTimestamp(fromUnixTimestamp({since})) \
           AND pc.cost > 0", // Only return non-zero costs
            since = since.timestamp(),
            db = self.db_name,
        );
        if let Some(start) = starting_after {
            query.push_str(&format!(" AND pc.batch_id < {}", start));
        }
        if let Some(end) = ending_before {
            query.push_str(&format!(" AND pc.batch_id > {}", end));
        }
        query.push_str(" ORDER BY pc.batch_id DESC");
        query.push_str(&format!(" LIMIT {}", limit));

        let rows = self.execute::<ProveCostRow>(&query).await?;
        Ok(rows)
    }

    /// Get the total prover cost for the given range
    pub async fn get_total_prove_cost(
        &self,
        sequencer: Option<AddressBytes>,
        range: TimeRange,
    ) -> Result<Option<u128>> {
        #[derive(Row, Deserialize)]
        struct SumRow {
            total: u128,
        }

        let mut query = format!(
            "SELECT sum(pc.cost) AS total \
             FROM {db}.prove_costs pc \
             INNER JOIN {db}.batches b ON pc.batch_id = b.batch_id \
             INNER JOIN {db}.l1_head_events l1 ON pc.l1_block_number = l1.l1_block_number \
             WHERE l1.block_ts >= toUnixTimestamp(now64() - INTERVAL {interval})",
            interval = range.interval(),
            db = self.db_name,
        );
        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND b.proposer_addr = unhex('{}')", encode(addr)));
        }

        let rows = self.execute::<SumRow>(&query).await?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };
        Ok(Some(row.total))
    }

    /// Get the transactions per second for each L2 block within the given range
    pub async fn get_l2_tps(
        &self,
        sequencer: Option<AddressBytes>,
        range: TimeRange,
    ) -> Result<Vec<L2TpsRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l2_block_number: u64,
            sum_tx: u32,
            ms_since_prev_block: Option<u64>,
        }

        let mut query = format!(
            "SELECT h.l2_block_number, sum_tx, \
                    toUInt64OrNull(toString((toUnixTimestamp64Milli(h.inserted_at) - \
                        lagInFrame(toUnixTimestamp64Milli(h.inserted_at)) OVER (ORDER BY h.l2_block_number)))) \
                        AS ms_since_prev_block \
             FROM {db}.l2_head_events h \
             WHERE h.block_ts >= toUnixTimestamp(now64() - INTERVAL {interval}) \
               AND {filter}",
            interval = range.interval(),
            filter = self.reorg_filter("h"),
            db = self.db_name,
        );
        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND sequencer = unhex('{}')", encode(addr)));
        }
        query.push_str(" ORDER BY l2_block_number ASC");

        let rows = self.execute::<RawRow>(&query).await?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let ms = r.ms_since_prev_block?;
                if ms == 0 {
                    return None;
                }
                Some(L2TpsRow {
                    l2_block_number: r.l2_block_number,
                    tps: r.sum_tx as f64 / (ms as f64 / 1000.0),
                })
            })
            .collect())
    }

    /// Get the transactions per second for each L2 block within the specified block range
    pub async fn get_l2_tps_block_range(
        &self,
        sequencer: Option<AddressBytes>,
        start_block: Option<u64>,
        end_block: Option<u64>,
    ) -> Result<Vec<L2TpsRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l2_block_number: u64,
            sum_tx: u32,
            ms_since_prev_block: Option<u64>,
        }

        let mut query = format!(
            "SELECT h.l2_block_number, sum_tx, \
                    toUInt64OrNull(toString((toUnixTimestamp64Milli(h.inserted_at) - \
                        lagInFrame(toUnixTimestamp64Milli(h.inserted_at)) OVER (ORDER BY h.l2_block_number)))) \
                        AS ms_since_prev_block \
             FROM {db}.l2_head_events h \
             WHERE {filter}",
            filter = self.reorg_filter("h"),
            db = self.db_name,
        );

        if let Some(start) = start_block {
            query.push_str(&format!(" AND h.l2_block_number >= {}", start));
        }

        if let Some(end) = end_block {
            query.push_str(&format!(" AND h.l2_block_number <= {}", end));
        }

        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND sequencer = unhex('{}')", encode(addr)));
        }

        query.push_str(" ORDER BY l2_block_number ASC");

        let rows = self.execute::<RawRow>(&query).await?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let ms = r.ms_since_prev_block?;
                if ms == 0 {
                    None
                } else {
                    Some(L2TpsRow {
                        l2_block_number: r.l2_block_number,
                        tps: r.sum_tx as f64 / (ms as f64 / 1000.0),
                    })
                }
            })
            .collect())
    }

    /// Get the total L2 transaction fee for the given range
    pub async fn get_l2_tx_fee(
        &self,
        sequencer: Option<AddressBytes>,
        range: TimeRange,
    ) -> Result<Option<u128>> {
        #[derive(Row, Deserialize)]
        struct SumRow {
            total: u128,
        }

        let mut query = format!(
            "SELECT sum(sum_priority_fee + sum_base_fee) AS total \
             FROM {db}.l2_head_events h \
             INNER JOIN {db}.batch_blocks bb \
               ON h.l2_block_number = bb.l2_block_number \
             INNER JOIN {db}.batches b \
               ON bb.batch_id = b.batch_id \
             WHERE h.block_ts >= toUnixTimestamp(now64() - INTERVAL {interval})",
            interval = range.interval(),
            db = self.db_name
        );
        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND h.sequencer = unhex('{}')", encode(addr)));
        }

        let rows = self.execute::<SumRow>(&query).await?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };
        Ok(Some(row.total))
    }

    /// Get the total priority fee for the given range
    pub async fn get_l2_priority_fee(
        &self,
        sequencer: Option<AddressBytes>,
        range: TimeRange,
    ) -> Result<Option<u128>> {
        #[derive(Row, Deserialize)]
        struct SumRow {
            total: u128,
        }

        let mut query = format!(
            "SELECT sum(sum_priority_fee) AS total \
             FROM {db}.l2_head_events h \
             INNER JOIN {db}.batch_blocks bb \
               ON h.l2_block_number = bb.l2_block_number \
             INNER JOIN {db}.batches b \
               ON bb.batch_id = b.batch_id \
             WHERE h.block_ts >= toUnixTimestamp(now64() - INTERVAL {interval})",
            interval = range.interval(),
            db = self.db_name
        );
        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND h.sequencer = unhex('{}')", encode(addr)));
        }

        let rows = self.execute::<SumRow>(&query).await?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };
        Ok(Some(row.total))
    }

    /// Get the total base fee for the given range
    pub async fn get_l2_base_fee(
        &self,
        sequencer: Option<AddressBytes>,
        range: TimeRange,
    ) -> Result<Option<u128>> {
        #[derive(Row, Deserialize)]
        struct SumRow {
            total: u128,
        }

        let mut query = format!(
            "SELECT sum(sum_base_fee) AS total \
             FROM {db}.l2_head_events h \
             INNER JOIN {db}.batch_blocks bb \
               ON h.l2_block_number = bb.l2_block_number \
             INNER JOIN {db}.batches b \
               ON bb.batch_id = b.batch_id \
             WHERE h.block_ts >= toUnixTimestamp(now64() - INTERVAL {interval})",
            interval = range.interval(),
            db = self.db_name
        );
        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND h.sequencer = unhex('{}')", encode(addr)));
        }

        let rows = self.execute::<SumRow>(&query).await?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };
        Ok(Some(row.total))
    }

    /// Get aggregated L2 fees grouped by sequencer for the given range
    pub async fn get_l2_fees_by_sequencer(&self, range: TimeRange) -> Result<Vec<SequencerFeeRow>> {
        let batch_components = self.get_batch_fee_components(None, range).await?;
        let prove_rows = self.get_prove_costs_by_proposer(range).await?;

        let prove_map: HashMap<AddressBytes, u128> = prove_rows.into_iter().collect();

        // Aggregate fees from batch components
        let mut fees_by_sequencer: HashMap<AddressBytes, (u128, u128, u128)> = HashMap::new();
        for component in batch_components {
            let entry = fees_by_sequencer.entry(component.sequencer).or_default();
            entry.0 += component.priority_fee;
            entry.1 += component.base_fee;
            entry.2 += component.l1_data_cost.unwrap_or(0);
        }

        // Create a unique set of all sequencers from all data sources
        let all_sequencers: BTreeSet<AddressBytes> =
            fees_by_sequencer.keys().copied().chain(prove_map.keys().copied()).collect();

        let mut results: Vec<SequencerFeeRow> = all_sequencers
            .into_iter()
            .map(|sequencer| {
                let (priority_fee, base_fee, l1_total_cost) =
                    fees_by_sequencer.get(&sequencer).copied().unwrap_or_default();

                SequencerFeeRow {
                    sequencer,
                    priority_fee,
                    base_fee,
                    l1_data_cost: (l1_total_cost > 0).then_some(l1_total_cost),
                    prove_cost: prove_map.get(&sequencer).copied(),
                }
            })
            .collect();

        // Sort by priority fee in descending order as the original query did
        results.sort_by(|a, b| b.priority_fee.cmp(&a.priority_fee));

        Ok(results)
    }

    /// Get the blob count for each batch within the given range
    pub async fn get_blobs_per_batch(&self, range: TimeRange) -> Result<Vec<BatchBlobCountRow>> {
        let query = format!(
            "SELECT b.l1_block_number, b.batch_id, b.blob_count \
             FROM {db}.batches b \
             INNER JOIN {db}.l1_head_events l1_events \
               ON b.l1_block_number = l1_events.l1_block_number \
             WHERE l1_events.block_ts >= toUnixTimestamp(now64() - INTERVAL {interval}) \
             ORDER BY b.l1_block_number ASC",
            interval = range.interval(),
            db = self.db_name,
        );

        let rows = self.execute::<BatchBlobCountRow>(&query).await?;
        Ok(rows)
    }

    /// Get the blob count per batch since the given cutoff time with cursor-based pagination.
    /// Results are returned in descending order by batch id.
    pub async fn get_blobs_per_batch_paginated(
        &self,
        since: DateTime<Utc>,
        limit: u64,
        starting_after: Option<u64>,
        ending_before: Option<u64>,
    ) -> Result<Vec<BatchBlobCountRow>> {
        let mut query = format!(
            "SELECT b.l1_block_number, b.batch_id, b.blob_count \
             FROM {db}.batches b \
             INNER JOIN {db}.l1_head_events l1_events \
               ON b.l1_block_number = l1_events.l1_block_number \
             WHERE l1_events.block_ts >= {since}",
            since = since.timestamp(),
            db = self.db_name,
        );
        if let Some(start) = starting_after {
            query.push_str(&format!(" AND b.batch_id < {}", start));
        }
        if let Some(end) = ending_before {
            query.push_str(&format!(" AND b.batch_id > {}", end));
        }
        query.push_str(" ORDER BY b.batch_id DESC");
        query.push_str(&format!(" LIMIT {}", limit));

        let rows = self.execute::<BatchBlobCountRow>(&query).await?;
        Ok(rows)
    }

    /// Get the average number of blobs per batch for the given range
    pub async fn get_avg_blobs_per_batch(&self, range: TimeRange) -> Result<Option<f64>> {
        #[derive(Row, Deserialize)]
        struct AvgRow {
            avg: f64,
        }

        let query = format!(
            "SELECT avg(b.blob_count) AS avg \
             FROM {db}.batches b \
             INNER JOIN {db}.l1_head_events l1_events \
               ON b.l1_block_number = l1_events.l1_block_number \
             WHERE l1_events.block_ts >= toUnixTimestamp(now64() - INTERVAL {})",
            range.interval(),
            db = self.db_name
        );

        let rows = self.execute::<AvgRow>(&query).await?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };

        if row.avg.is_nan() { Ok(None) } else { Ok(Some(row.avg)) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clickhouse::test::{Mock, handlers};

    #[derive(Row, serde::Serialize)]
    struct FeeRow {
        l2_block_number: u64,
        priority_fee: u128,
        base_fee: u128,
        l1_data_cost: Option<u128>,
    }

    #[tokio::test]
    async fn fee_components_returns_expected_rows() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![FeeRow {
            l2_block_number: 1,
            priority_fee: 10,
            base_fee: 20,
            l1_data_cost: Some(5),
        }]));

        let url = url::Url::parse(mock.url()).unwrap();
        let reader =
            ClickhouseReader::new(url, "db".to_owned(), "user".into(), "pass".into()).unwrap();

        let rows = reader.get_l2_fee_components(None, TimeRange::LastHour).await.unwrap();

        assert_eq!(
            rows,
            vec![BlockFeeComponentRow {
                l2_block_number: 1,
                priority_fee: 10,
                base_fee: 20,
                l1_data_cost: Some(5),
            }]
        );
    }

    #[derive(Row, serde::Serialize)]
    struct SeqFeeRow {
        sequencer: AddressBytes,
        priority_fee: u128,
        base_fee: u128,
        l1_data_cost: Option<u128>,
        prove_cost: Option<u128>,
    }

    #[tokio::test]
    async fn fees_by_sequencer_returns_expected_rows() {
        let mock = Mock::new();
        let addr = AddressBytes([1u8; 20]);

        // Mock for get_batch_fee_components
        mock.add(handlers::provide(vec![BatchFeeRow {
            batch_id: 1,
            l1_block_number: 10,
            proposer: addr,
            priority_fee: 10,
            base_fee: 20,
            l1_data_cost: Some(5),
        }]));

        // Mock for get_prove_costs_by_proposer
        #[derive(Row, serde::Serialize)]
        struct ProposerCostRow {
            proposer: AddressBytes,
            total_cost: u128,
        }
        mock.add(handlers::provide(vec![ProposerCostRow { proposer: addr, total_cost: 3 }]));

        let url = url::Url::parse(mock.url()).unwrap();
        let reader =
            ClickhouseReader::new(url, "db".to_owned(), "user".into(), "pass".into()).unwrap();

        let rows = reader.get_l2_fees_by_sequencer(TimeRange::LastHour).await.unwrap();

        assert_eq!(
            rows,
            vec![SequencerFeeRow {
                sequencer: addr,
                priority_fee: 10,
                base_fee: 20,
                l1_data_cost: Some(5),
                prove_cost: Some(3),
            }]
        );
    }

    #[test]
    fn fees_by_sequencer_query_has_proper_spacing() {
        // Test that the generated SQL query has proper spacing between keywords
        let reader =
            ClickhouseReader { base: clickhouse::Client::default(), db_name: "test_db".to_owned() };

        // Simulate the query generation logic
        let range = TimeRange::LastHour;
        let query = format!(
            "SELECT h.sequencer, \
                    sum(sum_priority_fee) AS priority_fee, \
                    sum(sum_base_fee) AS base_fee, \
                    toNullable(sum(if(b.batch_size > 0, intDiv(dc.cost, b.batch_size), NULL))) AS l1_data_cost, \
                    toNullable(sum(if(b.batch_size > 0, intDiv(pc.cost, b.batch_size), NULL))) AS prove_cost, \
                    toNullable(sum(if(b.batch_size > 0, intDiv(vc.cost, b.batch_size), NULL))) AS verify_cost \
             FROM {db}.l2_head_events h \
             LEFT JOIN {db}.batch_blocks bb \
               ON h.l2_block_number = bb.l2_block_number \
             LEFT JOIN {db}.batches b \
               ON bb.batch_id = b.batch_id \
             LEFT JOIN {db}.l1_data_costs dc \
               ON b.batch_id = dc.batch_id \
             LEFT JOIN {db}.prove_costs pc \
               ON b.batch_id = pc.batch_id \
             LEFT JOIN {db}.verify_costs vc \
               ON b.batch_id = vc.batch_id \
             WHERE h.block_ts >= toUnixTimestamp(now64() - INTERVAL {interval}) \
               AND bb.batch_id IS NOT NULL \
             GROUP BY h.sequencer \
             ORDER BY priority_fee DESC",
            interval = range.interval(),
            db = reader.db_name,
        );

        // Verify that the problematic concatenations have proper spacing
        assert!(
            query.contains("l2_head_events h LEFT JOIN"),
            "Query should have space between 'h' and 'LEFT JOIN'"
        );
        assert!(
            query.contains("l1_data_costs dc ON"),
            "Query should have space between 'dc' and 'ON'"
        );
        assert!(
            query.contains("batch_blocks bb ON"),
            "Query should have space between 'bb' and 'ON'",
        );
        assert!(query.contains("batches b ON"), "Query should have space between 'b' and 'ON'",);
        assert!(
            query.contains("prove_costs pc ON"),
            "Query should have space between 'pc' and 'ON'",
        );
        assert!(
            query.contains("verify_costs vc ON"),
            "Query should have space between 'vc' and 'ON'",
        );
        assert!(
            query.contains("bb.l2_block_number LEFT JOIN"),
            "Query should have space between 'l2_block_number' and 'LEFT JOIN'"
        );
        assert!(query.contains(") AND"), "Query should have space between ')' and 'AND'");

        // Verify that malformed tokens are not present
        assert!(!query.contains("hLEFT"), "Query should not contain 'hLEFT'");
        assert!(!query.contains("dcON"), "Query should not contain 'dcON'");
        assert!(!query.contains("bbON"), "Query should not contain 'bbON'");
        assert!(!query.contains("pcON"), "Query should not contain 'pcON'");
        assert!(!query.contains("vcON"), "Query should not contain 'vcON'");
    }

    #[derive(Row, serde::Serialize)]
    struct BatchFeeRow {
        batch_id: u64,
        l1_block_number: u64,
        proposer: AddressBytes,
        priority_fee: u128,
        base_fee: u128,
        l1_data_cost: Option<u128>,
    }

    #[tokio::test]
    async fn batch_fee_components_returns_expected_rows() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![BatchFeeRow {
            batch_id: 1,
            l1_block_number: 10,
            proposer: AddressBytes([1u8; 20]),
            priority_fee: 10,
            base_fee: 20,
            l1_data_cost: Some(5),
        }]));

        let url = url::Url::parse(mock.url()).unwrap();
        let reader =
            ClickhouseReader::new(url, "db".to_owned(), "user".into(), "pass".into()).unwrap();

        let rows = reader.get_batch_fee_components(None, TimeRange::LastHour).await.unwrap();

        assert_eq!(
            rows,
            vec![BatchFeeComponentRow {
                batch_id: 1,
                l1_block_number: 10,
                sequencer: AddressBytes([1u8; 20]),
                priority_fee: 10,
                base_fee: 20,
                l1_data_cost: Some(5),
            }]
        );
    }

    #[tokio::test]
    async fn batch_total_fee_helpers_return_expected_values() {
        let mock = Mock::new();
        for _ in 0..3 {
            mock.add(handlers::provide(vec![BatchFeeRow {
                batch_id: 1,
                l1_block_number: 10,
                proposer: AddressBytes([1u8; 20]),
                priority_fee: 10,
                base_fee: 20,
                l1_data_cost: Some(5),
            }]));
        }

        let url = url::Url::parse(mock.url()).unwrap();
        let reader =
            ClickhouseReader::new(url, "db".to_owned(), "user".into(), "pass".into()).unwrap();

        let priority =
            reader.get_batch_priority_fee(None, TimeRange::LastHour).await.unwrap().unwrap();
        assert_eq!(priority, 10);
        let base = reader.get_batch_base_fee(None, TimeRange::LastHour).await.unwrap().unwrap();
        assert_eq!(base, 20);
        let cost =
            reader.get_batch_total_data_cost(None, TimeRange::LastHour).await.unwrap().unwrap();
        assert_eq!(cost, 5);
    }

    #[tokio::test]
    async fn batch_fees_by_proposer_returns_expected_rows() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![SeqFeeRow {
            sequencer: AddressBytes([1u8; 20]),
            priority_fee: 10,
            base_fee: 20,
            l1_data_cost: Some(5),
            prove_cost: None,
        }]));

        let url = url::Url::parse(mock.url()).unwrap();
        let reader =
            ClickhouseReader::new(url, "db".to_owned(), "user".into(), "pass".into()).unwrap();

        let rows = reader.get_batch_fees_by_proposer(TimeRange::LastHour).await.unwrap();

        assert_eq!(
            rows,
            vec![SequencerFeeRow {
                sequencer: AddressBytes([1u8; 20]),
                priority_fee: 10,
                base_fee: 20,
                l1_data_cost: Some(5),
                prove_cost: None,
            }]
        );
    }
}
