//! `ClickHouse` reader functionality for API
//! Handles read-only operations and analytics queries

use chrono::{DateTime, LocalResult, TimeZone, Utc};
use clickhouse::{Client, Row};
use derive_more::Debug;
use eyre::{Context, Result};
use hex::encode;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::{debug, error};
use url::Url;

use crate::{
    models::{
        BatchBlobCountRow, BatchPostingTimeRow, BatchProveTimeRow, BatchVerifyTimeRow,
        BlockTransactionRow, ForcedInclusionProcessedRow, L1BlockTimeRow, L1DataCostRow,
        L2BlockTimeRow, L2GasUsedRow, L2ReorgRow, L2TpsRow, PreconfData, SequencerBlockRow,
        SequencerDistributionRow, SlashingEventRow,
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
    /// Maximum allowed range in seconds (7 days).
    const MAX_SECONDS: u64 = 7 * 24 * 3600;

    /// Create a [`TimeRange`] from a [`chrono::Duration`], clamping to the
    /// allowed maximum of seven days.
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
        let client = Client::default()
            .with_url(url)
            .with_database(db_name.clone())
            .with_user(username)
            .with_password(password);

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
        let query =
            format!("SELECT max(block_ts) AS block_ts FROM {}.l2_head_events", self.db_name);
        let rows = self.execute::<MaxTs>(&query).await.context("fetching max(block_ts) failed")?;
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
        let query =
            format!("SELECT max(block_ts) AS block_ts FROM {}.l1_head_events", self.db_name);

        let rows = self.execute::<MaxTs>(&query).await.context("fetching max(block_ts) failed")?;

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

        // Use ORDER BY + LIMIT 1 instead of max() for better performance on large tables
        // This approach can utilize indexes more efficiently
        let query = format!(
            "SELECT l2_block_number FROM {}.l2_head_events \
             ORDER BY l2_block_number DESC LIMIT 1",
            &self.db_name
        );

        let rows = self.execute::<BlockNumber>(&query).await?;
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

        // Use ORDER BY + LIMIT 1 instead of max() for better performance on large tables
        let query = format!(
            "SELECT l1_block_number FROM {}.l1_head_events \
             ORDER BY l1_block_number DESC LIMIT 1",
            &self.db_name
        );

        let rows = self.execute::<BlockNumber>(&query).await?;
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
        let query = format!(
            "SELECT max(l1_events.block_ts) AS block_ts
             FROM {db}.batches b
             INNER JOIN {db}.l1_head_events l1_events
               ON b.l1_block_number = l1_events.l1_block_number",
            db = &self.db_name
        );

        let rows = self
            .execute::<MaxTs>(&query)
            .await
            .context("fetching max batch L1 block timestamp failed")?;

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
        let query = format!(
            "SELECT slot, candidates, current_operator, next_operator FROM {}.preconf_data ORDER BY inserted_at DESC LIMIT 1",
            self.db_name
        );
        let rows = self.execute::<PreconfData>(&query).await?;
        Ok(rows.into_iter().next())
    }

    /// Get all batches that have not been proven and are older than the given cutoff time
    pub async fn get_unproved_batches_older_than(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<Vec<(u64, u64, DateTime<Utc>)>> {
        let query = format!(
            "SELECT b.l1_block_number, b.batch_id, toUnixTimestamp64Milli(b.inserted_at) as inserted_at \
             FROM (SELECT l1_block_number, batch_id, inserted_at \
                   FROM {db}.batches \
                   WHERE inserted_at < toDateTime64({}, 3)) AS b \
             LEFT JOIN {db}.proved_batches p \
               ON b.l1_block_number = p.l1_block_number AND b.batch_id = p.batch_id \
             WHERE p.batch_id IS NULL \
             ORDER BY b.inserted_at ASC",
            cutoff.timestamp_millis() as f64 / 1000.0,
            db = self.db_name
        );
        let rows = self
            .execute::<(u64, u64, u64)>(&query)
            .await
            .context("fetching unproved batches failed")?;
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
        let query = format!("SELECT batch_id FROM {}.proved_batches", self.db_name);
        let rows = self.execute::<ProvedBatchIdRow>(&query).await?;
        Ok(rows.into_iter().map(|r| r.batch_id).collect())
    }

    /// Get all batches that have not been verified and are older than the given cutoff time
    pub async fn get_unverified_batches_older_than(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<Vec<(u64, u64, DateTime<Utc>)>> {
        let query = format!(
            "SELECT b.l1_block_number, b.batch_id, toUnixTimestamp64Milli(b.inserted_at) as inserted_at \
             FROM (SELECT l1_block_number, batch_id, inserted_at \
                   FROM {db}.batches \
                   WHERE inserted_at < toDateTime64({}, 3)) AS b \
             LEFT JOIN {db}.verified_batches v \
               ON b.l1_block_number = v.l1_block_number AND b.batch_id = v.batch_id \
             WHERE v.batch_id IS NULL \
             ORDER BY b.inserted_at ASC",
            cutoff.timestamp_millis() as f64 / 1000.0,
            db = self.db_name
        );
        let rows = self
            .execute::<(u64, u64, u64)>(&query)
            .await
            .context("fetching unverified batches failed")?;
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
        let query = format!("SELECT batch_id FROM {}.verified_batches", self.db_name);
        let rows = self.execute::<VerifiedBatchIdRow>(&query).await?;
        Ok(rows.into_iter().map(|r| r.batch_id).collect())
    }

    /// Get all slashing events that occurred after the given cutoff time
    pub async fn get_slashing_events_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<SlashingEventRow>> {
        let query = format!(
            "SELECT l1_block_number, validator_addr FROM {}.slashing_events \
             WHERE inserted_at > toDateTime64({}, 3) \
             ORDER BY inserted_at ASC",
            self.db_name,
            since.timestamp_millis() as f64 / 1000.0,
        );
        let rows = self
            .execute::<SlashingEventRow>(&query)
            .await
            .context("fetching slashing events failed")?;
        Ok(rows)
    }

    /// Get all forced inclusion events that occurred after the given cutoff time
    pub async fn get_forced_inclusions_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<ForcedInclusionProcessedRow>> {
        let query = format!(
            "SELECT blob_hash FROM {}.forced_inclusion_processed \
             WHERE inserted_at > toDateTime64({}, 3) \
             ORDER BY inserted_at ASC",
            self.db_name,
            since.timestamp_millis() as f64 / 1000.0,
        );
        let rows = self
            .execute::<ForcedInclusionProcessedRow>(&query)
            .await
            .context("fetching forced inclusion events failed")?;
        Ok(rows)
    }

    /// Get all L2 reorg events that occurred after the given cutoff time
    pub async fn get_l2_reorgs_since(&self, since: DateTime<Utc>) -> Result<Vec<L2ReorgRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l2_block_number: u64,
            depth: u16,
            ts: u64,
        }

        let query = format!(
            "SELECT l2_block_number, depth, \
                    toUInt64(toUnixTimestamp64Milli(inserted_at)) AS ts \
             FROM {}.l2_reorgs \
             WHERE inserted_at > toDateTime64({}, 3) \
             ORDER BY inserted_at DESC",
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

        let query = format!(
            "SELECT candidates, current_operator, next_operator FROM {}.preconf_data \
             WHERE inserted_at > toDateTime64({}, 3)",
            self.db_name,
            since.timestamp_millis() as f64 / 1000.0,
        );
        let rows = self.execute::<GatewayRow>(&query).await?;
        let mut set = std::collections::HashSet::new();
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
             ORDER BY sequencer, h.l2_block_number DESC",
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
        let mut query = format!(
            "SELECT sequencer, h.l2_block_number, sum_tx \
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
        // Cap the limit to a reasonable default if not specified
        let actual_limit = if limit == u64::MAX { 1000 } else { limit };
        query.push_str(&format!(" LIMIT {}", actual_limit));

        let rows = self.execute::<BlockTransactionRow>(&query).await?;
        Ok(rows)
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

        let query = format!(
            "SELECT COALESCE(avg(verify_time_ms), 0) AS avg_ms \
             FROM {db}.batch_verify_times_mv \
             WHERE verified_at >= now64() - INTERVAL {interval}",
            interval = range.interval(),
            db = self.db_name
        );

        let rows = self.execute::<AvgRow>(&query).await?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };

        if row.avg_ms == 0.0 { Ok(None) } else { Ok(Some(row.avg_ms.round() as u64)) }
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
                    toUInt64OrNull(toString(ts - prev_ts)) AS ms_since_prev_batch \
             FROM ( \
                 SELECT b.batch_id AS batch_id, \
                        toUInt64(l1_events.block_ts * 1000) AS ts, \
                        lagInFrame(toUInt64(l1_events.block_ts * 1000)) \
                            OVER (ORDER BY l1_events.block_ts) AS prev_ts \
                   FROM {db}.batches b \
                   INNER JOIN {db}.l1_head_events l1_events \
                     ON b.l1_block_number = l1_events.l1_block_number \
                  WHERE l1_events.block_ts >= toUnixTimestamp(now64() - INTERVAL {interval}) \
                  ORDER BY l1_events.block_ts \
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

    /// Get prove times in seconds for batches proved within the given range
    pub async fn get_prove_times(&self, range: TimeRange) -> Result<Vec<BatchProveTimeRow>> {
        let mv_query = format!(
            "SELECT batch_id, toUInt64(prove_time_ms / 1000) AS seconds_to_prove \
             FROM {db}.batch_prove_times_mv \
             WHERE proved_at >= now64() - INTERVAL {interval} \
             ORDER BY batch_id ASC",
            interval = range.interval(),
            db = self.db_name,
        );

        let rows = self.execute::<BatchProveTimeRow>(&mv_query).await?;
        if !rows.is_empty() {
            return Ok(rows);
        }

        let fallback_query = format!(
            "SELECT b.batch_id AS batch_id, \
                    (l1_proved.block_ts - l1_proposed.block_ts) AS seconds_to_prove \
             FROM {db}.batches b \
             JOIN {db}.proved_batches pb ON b.batch_id = pb.batch_id \
             JOIN {db}.l1_head_events l1_proposed \
               ON b.l1_block_number = l1_proposed.l1_block_number \
             JOIN {db}.l1_head_events l1_proved \
               ON pb.l1_block_number = l1_proved.l1_block_number \
             WHERE l1_proved.block_ts >= (toUInt64(now()) - {secs}) \
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
             ORDER BY batch_id ASC",
            interval = range.interval(),
            db = self.db_name,
        );

        let rows = self.execute::<BatchVerifyTimeRow>(&mv_query).await?;
        if !rows.is_empty() {
            return Ok(rows);
        }

        let fallback_query = format!(
            "SELECT pb.batch_id AS batch_id, \
                    (l1_verified.block_ts - l1_proved.block_ts) AS seconds_to_verify \
             FROM {db}.proved_batches pb \
             INNER JOIN {db}.verified_batches vb \
                ON pb.batch_id = vb.batch_id AND pb.block_hash = vb.block_hash \
             INNER JOIN {db}.l1_head_events l1_proved \
                ON pb.l1_block_number = l1_proved.l1_block_number \
             INNER JOIN {db}.l1_head_events l1_verified \
                ON vb.l1_block_number = l1_verified.l1_block_number \
             WHERE l1_verified.block_ts >= (toUInt64(now()) - {secs}) \
               AND l1_verified.block_ts > l1_proved.block_ts \
               AND (l1_verified.block_ts - l1_proved.block_ts) > 60 \
             ORDER BY pb.batch_id ASC",
            secs = range.seconds(),
            db = self.db_name,
        );

        let rows = self.execute::<BatchVerifyTimeRow>(&fallback_query).await?;
        Ok(rows)
    }

    /// Get L1 block numbers grouped by minute for the given range
    pub async fn get_l1_block_times(&self, range: TimeRange) -> Result<Vec<L1BlockTimeRow>> {
        let query = format!(
            "SELECT toUInt64(toStartOfMinute(fromUnixTimestamp64Milli(block_ts * 1000))) AS minute, \
                    max(l1_block_number) AS block_number \
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
        query.push_str(" ORDER BY l2_block_number DESC");
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
            gas_used: u64,
        }

        let mut query = format!(
            "SELECT h.l2_block_number, toUInt64(sum_gas_used) AS gas_used \
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
            .map(|r| L2GasUsedRow { l2_block_number: r.l2_block_number, gas_used: r.gas_used })
            .collect())
    }

    /// Get the L1 data posting cost for each block within the given range
    pub async fn get_l1_data_costs(&self, range: TimeRange) -> Result<Vec<L1DataCostRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l1_block_number: u64,
            cost: u128,
        }

        let query = format!(
            "SELECT l1_block_number, cost \
             FROM {db}.l1_data_costs \
             WHERE l1_block_number IN (\
                 SELECT l1_block_number FROM {db}.l1_head_events \
                 WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL {interval})) \
             ORDER BY l1_block_number ASC",
            interval = range.interval(),
            db = self.db_name,
        );

        let rows = self.execute::<RawRow>(&query).await?;
        Ok(rows
            .into_iter()
            .map(|r| L1DataCostRow { l1_block_number: r.l1_block_number, cost: r.cost })
            .collect())
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
                    toUInt64OrNull(toString((h.block_ts - lagInFrame(h.block_ts) OVER (ORDER BY h.l2_block_number)) * 1000)) \
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
        query.push_str(" ORDER BY l2_block_number DESC");

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
            "SELECT sum(sum_priority_fee + toUInt128(sum_base_fee * 3 / 4)) AS total \
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

        let rows = self.execute::<SumRow>(&query).await?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };
        Ok(Some(row.total))
    }

    /// Get the blob count for each batch within the given range
    pub async fn get_blobs_per_batch(&self, range: TimeRange) -> Result<Vec<BatchBlobCountRow>> {
        let query = format!(
            "SELECT b.l1_block_number, b.batch_id, b.blob_count \
             FROM {db}.batches b \
             INNER JOIN {db}.l1_head_events l1_events \
               ON b.l1_block_number = l1_events.l1_block_number \
             WHERE l1_events.block_ts >= toUnixTimestamp(now64() - INTERVAL {interval}) \
             ORDER BY b.l1_block_number DESC",
            interval = range.interval(),
            db = self.db_name,
        );

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
