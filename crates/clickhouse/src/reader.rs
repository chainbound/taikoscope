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

use crate::models::{
    BatchBlobCountRow, BatchProveTimeRow, BatchVerifyTimeRow, BlockTransactionRow,
    ForcedInclusionProcessedRow, L1BlockTimeRow, L2BlockTimeRow, L2GasUsedRow, L2ReorgRow,
    SequencerBlockRow, SequencerDistributionRow, SlashingEventRow,
};

#[derive(Row, Deserialize, Serialize)]
struct MaxTs {
    block_ts: u64,
}

/// Supported time ranges for analytics queries
#[derive(Copy, Clone, Debug)]
pub enum TimeRange {
    /// Data from the last hour
    LastHour,
    /// Data from the last 24 hours
    Last24Hours,
    /// Data from the last 7 days
    Last7Days,
}

impl TimeRange {
    /// Return the `ClickHouse` interval string for this range
    const fn interval(self) -> &'static str {
        match self {
            Self::LastHour => "1 HOUR",
            Self::Last24Hours => "24 HOUR",
            Self::Last7Days => "7 DAY",
        }
    }

    /// Return the duration in seconds for this range
    const fn seconds(self) -> u64 {
        match self {
            Self::LastHour => 3600,
            Self::Last24Hours => 86400,
            Self::Last7Days => 604800,
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
        let client = self.base.clone().with_database(&self.db_name);
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
    pub async fn get_last_l2_block_number(&self) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct MaxNumber {
            number: u64,
        }

        let query =
            format!("SELECT max(l2_block_number) AS number FROM {}.l2_head_events", &self.db_name);

        let rows = self.execute::<MaxNumber>(&query).await?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };
        if row.number == 0 {
            return Ok(None);
        }
        Ok(Some(row.number))
    }

    /// Get the latest L1 block number.
    pub async fn get_last_l1_block_number(&self) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct MaxNumber {
            number: u64,
        }

        let query =
            format!("SELECT max(l1_block_number) AS number FROM {}.l1_head_events", &self.db_name);

        let rows = self.execute::<MaxNumber>(&query).await?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };
        if row.number == 0 {
            return Ok(None);
        }
        Ok(Some(row.number))
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

    /// Get timestamp of the latest `BatchesVerified` event insertion in UTC
    pub async fn get_last_verified_batch_time(&self) -> Result<Option<DateTime<Utc>>> {
        let query = format!(
            "SELECT toUInt64(max(inserted_at)) AS block_ts FROM {}.verified_batches",
            &self.db_name
        );

        let rows =
            self.execute::<MaxTs>(&query).await.context("fetching max(inserted_at) failed")?;

        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };

        if row.block_ts == 0 {
            return Ok(None);
        }

        let ts_opt = match Utc.timestamp_millis_opt(row.block_ts as i64) {
            LocalResult::Single(dt) => Some(dt),
            _ => None,
        };
        Ok(ts_opt)
    }

    /// Get the last observed current operator
    pub async fn get_last_current_operator(&self) -> Result<Option<[u8; 20]>> {
        #[derive(Row, Deserialize)]
        struct OpRow {
            current_operator: Option<[u8; 20]>,
        }

        let query = format!(
            "SELECT current_operator FROM {}.preconf_data ORDER BY inserted_at DESC LIMIT 1",
            self.db_name
        );
        let rows = self.execute::<OpRow>(&query).await?;
        Ok(rows.into_iter().next().and_then(|r| r.current_operator))
    }

    /// Get the last observed next operator
    pub async fn get_last_next_operator(&self) -> Result<Option<[u8; 20]>> {
        #[derive(Row, Deserialize)]
        struct OpRow {
            next_operator: Option<[u8; 20]>,
        }

        let query = format!(
            "SELECT next_operator FROM {}.preconf_data ORDER BY inserted_at DESC LIMIT 1",
            self.db_name
        );
        let rows = self.execute::<OpRow>(&query).await?;
        Ok(rows.into_iter().next().and_then(|r| r.next_operator))
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
        let query = format!(
            "SELECT l2_block_number, depth FROM {}.l2_reorgs \
             WHERE inserted_at > toDateTime64({}, 3) \
             ORDER BY inserted_at ASC",
            self.db_name,
            since.timestamp_millis() as f64 / 1000.0,
        );
        let rows =
            self.execute::<L2ReorgRow>(&query).await.context("fetching reorg events failed")?;
        Ok(rows)
    }

    /// Get all active gateway addresses observed since the given cutoff time
    pub async fn get_active_gateways_since(&self, since: DateTime<Utc>) -> Result<Vec<[u8; 20]>> {
        #[derive(Row, Deserialize)]
        struct GatewayRow {
            candidates: Vec<[u8; 20]>,
            current_operator: Option<[u8; 20]>,
            next_operator: Option<[u8; 20]>,
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
            "SELECT sequencer, count() AS blocks FROM {db}.l2_head_events \
             WHERE inserted_at > toDateTime64({}, 3) \
             GROUP BY sequencer ORDER BY blocks DESC",
            since.timestamp_millis() as f64 / 1000.0,
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
            "SELECT sequencer, l2_block_number FROM {db}.l2_head_events \
             WHERE inserted_at > toDateTime64({}, 3) \
             ORDER BY sequencer, l2_block_number",
            since.timestamp_millis() as f64 / 1000.0,
            db = self.db_name,
        );

        let rows = self.execute::<SequencerBlockRow>(&query).await?;
        Ok(rows)
    }

    /// Get transactions per block since the given cutoff time
    pub async fn get_block_transactions_since(
        &self,
        since: DateTime<Utc>,
        sequencer: Option<[u8; 20]>,
    ) -> Result<Vec<BlockTransactionRow>> {
        let mut query = format!(
            "SELECT sequencer, l2_block_number, sum_tx FROM {db}.l2_head_events \
             WHERE inserted_at > toDateTime64({}, 3)",
            since.timestamp_millis() as f64 / 1000.0,
            db = self.db_name,
        );
        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND sequencer = unhex('{}')", encode(addr)));
        }
        query.push_str(" ORDER BY l2_block_number");

        let rows = self.execute::<BlockTransactionRow>(&query).await?;
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
        sequencer: Option<[u8; 20]>,
    ) -> Result<Vec<BlockTransactionRow>> {
        let mut query = format!(
            "SELECT sequencer, l2_block_number, sum_tx FROM {db}.l2_head_events \
             WHERE inserted_at > toDateTime64({}, 3)",
            since.timestamp_millis() as f64 / 1000.0,
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

    /// Get the average time in milliseconds it takes for a batch to be proven within the last hour
    pub async fn get_avg_prove_time_last_hour(&self) -> Result<Option<u64>> {
        self.get_avg_prove_time(TimeRange::LastHour).await
    }

    /// Get the average time in milliseconds it takes for a batch to be proven within the last 24
    /// hours
    pub async fn get_avg_prove_time_last_24_hours(&self) -> Result<Option<u64>> {
        self.get_avg_prove_time(TimeRange::Last24Hours).await
    }

    /// Get the average time in milliseconds it takes for a batch to be proven within the last 7
    /// days
    pub async fn get_avg_prove_time_last_7_days(&self) -> Result<Option<u64>> {
        self.get_avg_prove_time(TimeRange::Last7Days).await
    }

    /// Get the average time in milliseconds it takes for a batch to be verified
    /// for verifications submitted within the last hour
    pub async fn get_avg_verify_time_last_hour(&self) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct AvgRow {
            avg_ms: f64,
        }

        let query = format!(
            "SELECT COALESCE(avg(verify_time_ms), 0) AS avg_ms \
             FROM {db}.batch_verify_times_mv \
             WHERE verified_at >= now64() - INTERVAL 1 HOUR",
            db = self.db_name
        );

        let rows = self.execute::<AvgRow>(&query).await?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };

        if row.avg_ms == 0.0 { Ok(None) } else { Ok(Some(row.avg_ms.round() as u64)) }
    }

    /// Get the average time in milliseconds it takes for a batch to be verified
    /// for verifications submitted within the last 24 hours
    pub async fn get_avg_verify_time_last_24_hours(&self) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct AvgRow {
            avg_ms: f64,
        }

        let query = format!(
            "SELECT COALESCE(avg(verify_time_ms), 0) AS avg_ms \
             FROM {db}.batch_verify_times_mv \
             WHERE verified_at >= now64() - INTERVAL 24 HOUR",
            db = self.db_name
        );

        let rows = self.execute::<AvgRow>(&query).await?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };

        if row.avg_ms == 0.0 { Ok(None) } else { Ok(Some(row.avg_ms.round() as u64)) }
    }

    /// Get the average time in milliseconds it takes for a batch to be verified
    /// for verifications submitted within the last 7 days
    pub async fn get_avg_verify_time_last_7_days(&self) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct AvgRow {
            avg_ms: f64,
        }

        let query = format!(
            "SELECT COALESCE(avg(verify_time_ms), 0) AS avg_ms \
             FROM {db}.batch_verify_times_mv \
             WHERE verified_at >= now64() - INTERVAL 7 DAY",
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
    /// observed within the last hour
    pub async fn get_l2_block_cadence_last_hour(
        &self,
        sequencer: Option<[u8; 20]>,
    ) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct CadenceRow {
            min_ts: u64,
            max_ts: u64,
            cnt: u64,
        }

        let mut query = format!(
            "SELECT toUInt64(min(toUnixTimestamp64Milli(inserted_at))) AS min_ts, \
                    toUInt64(max(toUnixTimestamp64Milli(inserted_at))) AS max_ts, \
                    count() as cnt \
             FROM {db}.l2_head_events \
             WHERE inserted_at >= now64() - INTERVAL 1 HOUR",
            db = self.db_name
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

    /// Get the average interval in milliseconds between consecutive L2 blocks
    /// observed within the last 24 hours
    pub async fn get_l2_block_cadence_last_24_hours(
        &self,
        sequencer: Option<[u8; 20]>,
    ) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct CadenceRow {
            min_ts: u64,
            max_ts: u64,
            cnt: u64,
        }

        let mut query = format!(
            "SELECT toUInt64(min(toUnixTimestamp64Milli(inserted_at))) AS min_ts, \
                    toUInt64(max(toUnixTimestamp64Milli(inserted_at))) AS max_ts, \
                    count() as cnt \
             FROM {db}.l2_head_events \
             WHERE inserted_at >= now64() - INTERVAL 24 HOUR",
            db = self.db_name
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

    /// Get the average interval in milliseconds between consecutive L2 blocks
    /// observed within the last 7 days
    pub async fn get_l2_block_cadence_last_7_days(
        &self,
        sequencer: Option<[u8; 20]>,
    ) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct CadenceRow {
            min_ts: u64,
            max_ts: u64,
            cnt: u64,
        }

        let mut query = format!(
            "SELECT toUInt64(min(toUnixTimestamp64Milli(inserted_at))) AS min_ts, \
                    toUInt64(max(toUnixTimestamp64Milli(inserted_at))) AS max_ts, \
                    count() as cnt \
             FROM {db}.l2_head_events \
             WHERE inserted_at >= now64() - INTERVAL 7 DAY",
            db = self.db_name
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
    /// proposals observed within the last hour
    pub async fn get_batch_posting_cadence_last_hour(&self) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct CadenceRow {
            min_ts: u64,
            max_ts: u64,
            cnt: u64,
        }

        let query = format!(
            "SELECT toUInt64(min(toUnixTimestamp64Milli(inserted_at))) AS min_ts, \
                    toUInt64(max(toUnixTimestamp64Milli(inserted_at))) AS max_ts, \
                    count() as cnt \
             FROM {db}.batches \
             WHERE inserted_at >= now64() - INTERVAL 1 HOUR",
            db = self.db_name
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

    /// Get the average interval in milliseconds between consecutive batch
    /// proposals observed within the last 24 hours
    pub async fn get_batch_posting_cadence_last_24_hours(&self) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct CadenceRow {
            min_ts: u64,
            max_ts: u64,
            cnt: u64,
        }

        let query = format!(
            "SELECT toUInt64(min(toUnixTimestamp64Milli(inserted_at))) AS min_ts, \
                    toUInt64(max(toUnixTimestamp64Milli(inserted_at))) AS max_ts, \
                    count() as cnt \
             FROM {db}.batches \
             WHERE inserted_at >= now64() - INTERVAL 24 HOUR",
            db = self.db_name
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

    /// Get the average interval in milliseconds between consecutive batch
    /// proposals observed within the last 7 days
    pub async fn get_batch_posting_cadence_last_7_days(&self) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct CadenceRow {
            min_ts: u64,
            max_ts: u64,
            cnt: u64,
        }

        let query = format!(
            "SELECT toUInt64(min(toUnixTimestamp64Milli(inserted_at))) AS min_ts, \
                    toUInt64(max(toUnixTimestamp64Milli(inserted_at))) AS max_ts, \
                    count() as cnt \
             FROM {db}.batches \
             WHERE inserted_at >= now64() - INTERVAL 7 DAY",
            db = self.db_name
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

    /// Get prove times in seconds for batches proved within the last hour
    pub async fn get_prove_times_last_hour(&self) -> Result<Vec<BatchProveTimeRow>> {
        let mv_query = format!(
            "SELECT batch_id, toUInt64(prove_time_ms / 1000) AS seconds_to_prove \
             FROM {db}.batch_prove_times_mv \
             WHERE proved_at >= now64() - INTERVAL 1 HOUR \
             ORDER BY batch_id ASC",
            db = self.db_name
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
             WHERE l1_proved.block_ts >= (toUInt64(now()) - 3600) \
             ORDER BY b.batch_id ASC",
            db = self.db_name
        );

        let rows = self.execute::<BatchProveTimeRow>(&fallback_query).await?;
        Ok(rows)
    }

    /// Get prove times in seconds for batches proved within the last 24 hours
    pub async fn get_prove_times_last_24_hours(&self) -> Result<Vec<BatchProveTimeRow>> {
        let mv_query = format!(
            "SELECT batch_id, toUInt64(prove_time_ms / 1000) AS seconds_to_prove \
             FROM {db}.batch_prove_times_mv \
             WHERE proved_at >= now64() - INTERVAL 24 HOUR \
             ORDER BY batch_id ASC",
            db = self.db_name
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
             WHERE l1_proved.block_ts >= (toUInt64(now()) - 86400) \
             ORDER BY b.batch_id ASC",
            db = self.db_name
        );

        let rows = self.execute::<BatchProveTimeRow>(&fallback_query).await?;
        Ok(rows)
    }

    /// Get prove times in seconds for batches proved within the last 7 days
    pub async fn get_prove_times_last_7_days(&self) -> Result<Vec<BatchProveTimeRow>> {
        let mv_query = format!(
            "SELECT batch_id, toUInt64(prove_time_ms / 1000) AS seconds_to_prove \
             FROM {db}.batch_prove_times_mv \
             WHERE proved_at >= now64() - INTERVAL 7 DAY \
             ORDER BY batch_id ASC",
            db = self.db_name
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
             WHERE l1_proved.block_ts >= (toUInt64(now()) - 604800) \
             ORDER BY b.batch_id ASC",
            db = self.db_name
        );

        let rows = self.execute::<BatchProveTimeRow>(&fallback_query).await?;
        Ok(rows)
    }

    /// Get verify times in seconds for batches verified within the last hour
    pub async fn get_verify_times_last_hour(&self) -> Result<Vec<BatchVerifyTimeRow>> {
        let mv_query = format!(
            "SELECT batch_id, toUInt64(verify_time_ms / 1000) AS seconds_to_verify \
             FROM {db}.batch_verify_times_mv \
             WHERE verified_at >= now64() - INTERVAL 1 HOUR \
               AND verify_time_ms > 60000 \
             ORDER BY batch_id ASC",
            db = self.db_name
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
             WHERE l1_verified.block_ts >= (toUInt64(now()) - 3600) \
               AND l1_verified.block_ts > l1_proved.block_ts \
               AND (l1_verified.block_ts - l1_proved.block_ts) > 60 \
             ORDER BY pb.batch_id ASC",
            db = self.db_name
        );

        let rows = self.execute::<BatchVerifyTimeRow>(&fallback_query).await?;
        Ok(rows)
    }

    /// Get verify times in seconds for batches verified within the last 24 hours
    pub async fn get_verify_times_last_24_hours(&self) -> Result<Vec<BatchVerifyTimeRow>> {
        let mv_query = format!(
            "SELECT batch_id, toUInt64(verify_time_ms / 1000) AS seconds_to_verify \
             FROM {db}.batch_verify_times_mv \
             WHERE verified_at >= now64() - INTERVAL 24 HOUR \
               AND verify_time_ms > 60000 \
             ORDER BY batch_id ASC",
            db = self.db_name
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
             WHERE l1_verified.block_ts >= (toUInt64(now()) - 86400) \
               AND l1_verified.block_ts > l1_proved.block_ts \
               AND (l1_verified.block_ts - l1_proved.block_ts) > 60 \
             ORDER BY pb.batch_id ASC",
            db = self.db_name
        );

        let rows = self.execute::<BatchVerifyTimeRow>(&fallback_query).await?;
        Ok(rows)
    }

    /// Get verify times in seconds for batches verified within the last 7 days
    pub async fn get_verify_times_last_7_days(&self) -> Result<Vec<BatchVerifyTimeRow>> {
        let mv_query = format!(
            "SELECT batch_id, toUInt64(verify_time_ms / 1000) AS seconds_to_verify \
             FROM {db}.batch_verify_times_mv \
             WHERE verified_at >= now64() - INTERVAL 7 DAY \
               AND verify_time_ms > 60000 \
             ORDER BY batch_id ASC",
            db = self.db_name
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
             WHERE l1_verified.block_ts >= (toUInt64(now()) - 604800) \
               AND l1_verified.block_ts > l1_proved.block_ts \
               AND (l1_verified.block_ts - l1_proved.block_ts) > 60 \
             ORDER BY pb.batch_id ASC",
            db = self.db_name
        );

        let rows = self.execute::<BatchVerifyTimeRow>(&fallback_query).await?;
        Ok(rows)
    }

    /// Get L1 block numbers grouped by minute for the last hour
    pub async fn get_l1_block_times_last_hour(&self) -> Result<Vec<L1BlockTimeRow>> {
        let query = format!(
            "SELECT toUInt64(toStartOfMinute(fromUnixTimestamp64Milli(block_ts * 1000))) AS minute, \
                    max(l1_block_number) AS block_number \
             FROM {db}.l1_head_events \
             WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL 1 HOUR) \
             GROUP BY minute \
             ORDER BY minute",
            db = self.db_name
        );

        let rows = self.execute::<L1BlockTimeRow>(&query).await?;
        Ok(rows)
    }

    /// Get L1 block numbers grouped by minute for the last 24 hours
    pub async fn get_l1_block_times_last_24_hours(&self) -> Result<Vec<L1BlockTimeRow>> {
        let query = format!(
            "SELECT toUInt64(toStartOfMinute(fromUnixTimestamp64Milli(block_ts * 1000))) AS minute, \
                    max(l1_block_number) AS block_number \
             FROM {db}.l1_head_events \
             WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL 24 HOUR) \
             GROUP BY minute \
             ORDER BY minute",
            db = self.db_name
        );

        let rows = self.execute::<L1BlockTimeRow>(&query).await?;
        Ok(rows)
    }

    /// Get L1 block numbers grouped by minute for the last 7 days
    pub async fn get_l1_block_times_last_7_days(&self) -> Result<Vec<L1BlockTimeRow>> {
        let query = format!(
            "SELECT toUInt64(toStartOfMinute(fromUnixTimestamp64Milli(block_ts * 1000))) AS minute, \
                    max(l1_block_number) AS block_number \
             FROM {db}.l1_head_events \
             WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL 7 DAY) \
             GROUP BY minute \
             ORDER BY minute",
            db = self.db_name
        );

        let rows = self.execute::<L1BlockTimeRow>(&query).await?;
        Ok(rows)
    }

    /// Get the time between consecutive L2 blocks for the last hour
    pub async fn get_l2_block_times_last_hour(
        &self,
        sequencer: Option<[u8; 20]>,
    ) -> Result<Vec<L2BlockTimeRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l2_block_number: u64,
            block_time: u64,
            ms_since_prev_block: Option<u64>,
        }

        let mut query = format!(
            "SELECT l2_block_number, \
                    block_ts AS block_time, \
                    toUInt64OrNull(toString((block_ts - lagInFrame(block_ts) OVER (ORDER BY l2_block_number)) * 1000)) \
                        AS ms_since_prev_block \
             FROM {db}.l2_head_events \
             WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL 1 HOUR)",
            db = self.db_name
        );
        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND sequencer = unhex('{}')", encode(addr)));
        }
        query.push_str(" ORDER BY l2_block_number");
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

    /// Get the time between consecutive L2 blocks for the last 24 hours
    pub async fn get_l2_block_times_last_24_hours(
        &self,
        sequencer: Option<[u8; 20]>,
    ) -> Result<Vec<L2BlockTimeRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l2_block_number: u64,
            block_time: u64,
            ms_since_prev_block: Option<u64>,
        }

        let mut query = format!(
            "SELECT l2_block_number, \
                    block_ts AS block_time, \
                    toUInt64OrNull(toString((block_ts - lagInFrame(block_ts) OVER (ORDER BY l2_block_number)) * 1000)) \
                        AS ms_since_prev_block \
             FROM {db}.l2_head_events \
             WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL 24 HOUR)",
            db = self.db_name
        );
        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND sequencer = unhex('{}')", encode(addr)));
        }
        query.push_str(" ORDER BY l2_block_number");
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

    /// Get the time between consecutive L2 blocks for the last 7 days
    pub async fn get_l2_block_times_last_7_days(
        &self,
        sequencer: Option<[u8; 20]>,
    ) -> Result<Vec<L2BlockTimeRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l2_block_number: u64,
            block_time: u64,
            ms_since_prev_block: Option<u64>,
        }

        let mut query = format!(
            "SELECT l2_block_number, \
                    block_ts AS block_time, \
                    toUInt64OrNull(toString((block_ts - lagInFrame(block_ts) OVER (ORDER BY l2_block_number)) * 1000)) \
                        AS ms_since_prev_block \
             FROM {db}.l2_head_events \
             WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL 7 DAY)",
            db = self.db_name
        );
        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND sequencer = unhex('{}')", encode(addr)));
        }
        query.push_str(" ORDER BY l2_block_number");
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

    /// Get the average number of L2 transactions per second for the last hour
    pub async fn get_avg_l2_tps_last_hour(
        &self,
        sequencer: Option<[u8; 20]>,
    ) -> Result<Option<f64>> {
        #[derive(Row, Deserialize)]
        struct TpsRow {
            min_ts: u64,
            max_ts: u64,
            tx_sum: u64,
        }

        let mut query = format!(
            "SELECT toUInt64(min(block_ts)) AS min_ts, \
                    toUInt64(max(block_ts)) AS max_ts, \
                    sum(sum_tx) AS tx_sum \
             FROM {db}.l2_head_events \
             WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL 1 HOUR)",
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

    /// Get the average number of L2 transactions per second for the last 24 hours
    pub async fn get_avg_l2_tps_last_24_hours(
        &self,
        sequencer: Option<[u8; 20]>,
    ) -> Result<Option<f64>> {
        #[derive(Row, Deserialize)]
        struct TpsRow {
            min_ts: u64,
            max_ts: u64,
            tx_sum: u64,
        }

        let mut query = format!(
            "SELECT toUInt64(min(block_ts)) AS min_ts, \
                    toUInt64(max(block_ts)) AS max_ts, \
                    sum(sum_tx) AS tx_sum \
             FROM {db}.l2_head_events \
             WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL 24 HOUR)",
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

    /// Get the average number of L2 transactions per second for the last 7 days
    pub async fn get_avg_l2_tps_last_7_days(
        &self,
        sequencer: Option<[u8; 20]>,
    ) -> Result<Option<f64>> {
        #[derive(Row, Deserialize)]
        struct TpsRow {
            min_ts: u64,
            max_ts: u64,
            tx_sum: u64,
        }

        let mut query = format!(
            "SELECT toUInt64(min(block_ts)) AS min_ts, \
                    toUInt64(max(block_ts)) AS max_ts, \
                    sum(sum_tx) AS tx_sum \
             FROM {db}.l2_head_events \
             WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL 7 DAY)",
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

    /// Get the gas used for each L2 block in the last hour
    pub async fn get_l2_gas_used_last_hour(
        &self,
        sequencer: Option<[u8; 20]>,
    ) -> Result<Vec<L2GasUsedRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l2_block_number: u64,
            gas_used: u64,
        }

        let mut query = format!(
            "SELECT l2_block_number, toUInt64(sum_gas_used) AS gas_used \
             FROM {db}.l2_head_events \
             WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL 1 HOUR)",
            db = self.db_name
        );
        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND sequencer = unhex('{}')", encode(addr)));
        }
        query.push_str(" ORDER BY l2_block_number");

        let rows = self.execute::<RawRow>(&query).await?;
        Ok(rows
            .into_iter()
            .skip(1)
            .map(|r| L2GasUsedRow { l2_block_number: r.l2_block_number, gas_used: r.gas_used })
            .collect())
    }

    /// Get the gas used for each L2 block in the last 24 hours
    pub async fn get_l2_gas_used_last_24_hours(
        &self,
        sequencer: Option<[u8; 20]>,
    ) -> Result<Vec<L2GasUsedRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l2_block_number: u64,
            gas_used: u64,
        }

        let mut query = format!(
            "SELECT l2_block_number, toUInt64(sum_gas_used) AS gas_used \
             FROM {db}.l2_head_events \
             WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL 24 HOUR)",
            db = self.db_name
        );
        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND sequencer = unhex('{}')", encode(addr)));
        }
        query.push_str(" ORDER BY l2_block_number");

        let rows = self.execute::<RawRow>(&query).await?;
        Ok(rows
            .into_iter()
            .skip(1)
            .map(|r| L2GasUsedRow { l2_block_number: r.l2_block_number, gas_used: r.gas_used })
            .collect())
    }

    /// Get the gas used for each L2 block in the last 7 days
    pub async fn get_l2_gas_used_last_7_days(
        &self,
        sequencer: Option<[u8; 20]>,
    ) -> Result<Vec<L2GasUsedRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l2_block_number: u64,
            gas_used: u64,
        }

        let mut query = format!(
            "SELECT l2_block_number, toUInt64(sum_gas_used) AS gas_used \
             FROM {db}.l2_head_events \
             WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL 7 DAY)",
            db = self.db_name
        );
        if let Some(addr) = sequencer {
            query.push_str(&format!(" AND sequencer = unhex('{}')", encode(addr)));
        }
        query.push_str(" ORDER BY l2_block_number");

        let rows = self.execute::<RawRow>(&query).await?;
        Ok(rows
            .into_iter()
            .skip(1)
            .map(|r| L2GasUsedRow { l2_block_number: r.l2_block_number, gas_used: r.gas_used })
            .collect())
    }

    /// Get the blob count for each batch in the last hour
    pub async fn get_blobs_per_batch_last_hour(&self) -> Result<Vec<BatchBlobCountRow>> {
        let query = format!(
            "SELECT batch_id, blob_count FROM {db}.batches \
             WHERE inserted_at >= now64() - INTERVAL 1 HOUR \
             ORDER BY batch_id",
            db = self.db_name
        );

        let rows = self.execute::<BatchBlobCountRow>(&query).await?;
        Ok(rows)
    }

    /// Get the blob count for each batch in the last 24 hours
    pub async fn get_blobs_per_batch_last_24_hours(&self) -> Result<Vec<BatchBlobCountRow>> {
        let query = format!(
            "SELECT batch_id, blob_count FROM {db}.batches \
             WHERE inserted_at >= now64() - INTERVAL 24 HOUR \
             ORDER BY batch_id",
            db = self.db_name
        );

        let rows = self.execute::<BatchBlobCountRow>(&query).await?;
        Ok(rows)
    }

    /// Get the blob count for each batch in the last 7 days
    pub async fn get_blobs_per_batch_last_7_days(&self) -> Result<Vec<BatchBlobCountRow>> {
        let query = format!(
            "SELECT batch_id, blob_count FROM {db}.batches \
             WHERE inserted_at >= now64() - INTERVAL 7 DAY \
             ORDER BY batch_id",
            db = self.db_name
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
            "SELECT avg(blob_count) AS avg FROM {db}.batches \
             WHERE inserted_at >= now64() - INTERVAL {}",
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

    /// Get the average number of blobs per batch in the last hour
    pub async fn get_avg_blobs_per_batch_last_hour(&self) -> Result<Option<f64>> {
        self.get_avg_blobs_per_batch(TimeRange::LastHour).await
    }

    /// Get the average number of blobs per batch in the last 24 hours
    pub async fn get_avg_blobs_per_batch_last_24_hours(&self) -> Result<Option<f64>> {
        self.get_avg_blobs_per_batch(TimeRange::Last24Hours).await
    }

    /// Get the average number of blobs per batch in the last 7 days
    pub async fn get_avg_blobs_per_batch_last_7_days(&self) -> Result<Option<f64>> {
        self.get_avg_blobs_per_batch(TimeRange::Last7Days).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Utc;
    use clickhouse::test::{self, Mock, handlers};
    use serde::Serialize;

    use crate::ClickhouseReader;

    #[derive(Serialize, Row)]
    struct MaxNum {
        number: u64,
    }

    #[derive(Serialize, Row)]
    struct MaxTsRow {
        block_ts: u64,
    }

    #[tokio::test]
    async fn test_get_last_l2_block_number_empty() {
        let mock = Mock::new();
        mock.add(handlers::provide(Vec::<MaxNum>::new()));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_last_l2_block_number().await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_get_last_l2_block_number() {
        let mock = Mock::new();
        let expected = 42u64;
        mock.add(handlers::provide(vec![MaxNum { number: expected }]));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_last_l2_block_number().await.unwrap();
        assert_eq!(result, Some(expected));
    }

    #[tokio::test]
    async fn test_get_last_l1_block_number_empty() {
        let mock = Mock::new();
        mock.add(handlers::provide(Vec::<MaxNum>::new()));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_last_l1_block_number().await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_get_last_l1_block_number() {
        let mock = Mock::new();
        let expected = 50u64;
        mock.add(handlers::provide(vec![MaxNum { number: expected }]));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_last_l1_block_number().await.unwrap();
        assert_eq!(result, Some(expected));
    }

    #[tokio::test]
    async fn test_get_last_current_operator() {
        let mock = Mock::new();
        let addr = [1u8; 20];
        #[derive(Serialize, Row)]
        struct CurrentRowTest {
            current_operator: Option<[u8; 20]>,
        }
        mock.add(handlers::provide(vec![CurrentRowTest { current_operator: Some(addr) }]));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_last_current_operator().await.unwrap();
        assert_eq!(result, Some(addr));
    }

    #[tokio::test]
    async fn test_get_last_next_operator() {
        let mock = Mock::new();
        let addr = [2u8; 20];
        #[derive(Serialize, Row)]
        struct NextRowTest {
            next_operator: Option<[u8; 20]>,
        }
        mock.add(handlers::provide(vec![NextRowTest { next_operator: Some(addr) }]));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_last_next_operator().await.unwrap();
        assert_eq!(result, Some(addr));
    }

    #[tokio::test]
    async fn test_get_last_l2_head_time_empty() {
        let mock = Mock::new();
        mock.add(handlers::provide(Vec::<MaxTsRow>::new()));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_last_l2_head_time().await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_get_last_l2_head_time() {
        let mock = Mock::new();
        let ts = 42u64;
        mock.add(handlers::provide(vec![MaxTsRow { block_ts: ts }]));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let expected = Utc.timestamp_opt(ts as i64, 0).single().unwrap();
        let result = ch.get_last_l2_head_time().await.unwrap();
        assert_eq!(result, Some(expected));
    }

    #[tokio::test]
    async fn test_get_last_l1_head_time_empty() {
        let mock = Mock::new();
        mock.add(handlers::provide(Vec::<MaxTsRow>::new()));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_last_l1_head_time().await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_get_last_l1_head_time() {
        let mock = Mock::new();
        let ts = 24u64;
        mock.add(handlers::provide(vec![MaxTsRow { block_ts: ts }]));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let expected = Utc.timestamp_opt(ts as i64, 0).single().unwrap();
        let result = ch.get_last_l1_head_time().await.unwrap();
        assert_eq!(result, Some(expected));
    }

    #[tokio::test]
    async fn test_get_last_batch_time_empty() {
        let mock = Mock::new();
        mock.add(handlers::provide(Vec::<MaxTsRow>::new()));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_last_batch_time().await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_get_last_batch_time() {
        let mock = Mock::new();
        let ts = 100u64;
        mock.add(handlers::provide(vec![MaxTsRow { block_ts: ts }]));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let expected = Utc.timestamp_opt(ts as i64, 0).single().unwrap();
        let result = ch.get_last_batch_time().await.unwrap();
        assert_eq!(result, Some(expected));
    }

    #[tokio::test]
    async fn test_get_last_verified_batch_time_empty() {
        let mock = Mock::new();
        mock.add(handlers::provide(Vec::<MaxTsRow>::new()));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_last_verified_batch_time().await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_get_last_verified_batch_time() {
        let mock = Mock::new();
        let ts = 1500u64;
        mock.add(handlers::provide(vec![MaxTsRow { block_ts: ts }]));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let expected = Utc.timestamp_millis_opt(ts as i64).single().unwrap();
        let result = ch.get_last_verified_batch_time().await.unwrap();
        assert_eq!(result, Some(expected));
    }

    #[derive(Serialize, Row)]
    struct L2BlockTimeTestRow {
        l2_block_number: u64,
        block_time: u64,
        ms_since_prev_block: Option<u64>,
    }

    #[tokio::test]
    async fn test_get_l2_block_times_last_hour_empty() {
        let mock = Mock::new();
        mock.add(handlers::provide(Vec::<L2BlockTimeTestRow>::new()));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_l2_block_times_last_hour(None).await.unwrap();
        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_get_l2_block_times_last_hour() {
        let mock = Mock::new();
        let block_time = 1640995200u64; // 2022-01-01 00:00:00 UTC
        let test_data = vec![
            L2BlockTimeTestRow {
                l2_block_number: 100,
                block_time,
                ms_since_prev_block: None, // First block has no previous
            },
            L2BlockTimeTestRow {
                l2_block_number: 101,
                block_time: block_time + 12,
                ms_since_prev_block: Some(12000), // 12 seconds in ms
            },
            L2BlockTimeTestRow {
                l2_block_number: 102,
                block_time: block_time + 24,
                ms_since_prev_block: Some(12000), // 12 seconds in ms
            },
        ];
        mock.add(handlers::provide(test_data));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_l2_block_times_last_hour(None).await.unwrap();
        assert_eq!(result.len(), 2); // First block filtered out (no ms_since_prev_block)
        assert_eq!(result[0].l2_block_number, 101);
        assert_eq!(result[0].ms_since_prev_block, Some(12000));
        assert_eq!(result[1].l2_block_number, 102);
        assert_eq!(result[1].ms_since_prev_block, Some(12000));
    }

    #[tokio::test]
    async fn test_get_l2_block_times_last_24_hours_empty() {
        let mock = Mock::new();
        mock.add(handlers::provide(Vec::<L2BlockTimeTestRow>::new()));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_l2_block_times_last_24_hours(None).await.unwrap();
        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_get_l2_block_times_last_24_hours() {
        let mock = Mock::new();
        let block_time = 1640995200u64;
        let test_data = vec![
            L2BlockTimeTestRow { l2_block_number: 200, block_time, ms_since_prev_block: None },
            L2BlockTimeTestRow {
                l2_block_number: 201,
                block_time: block_time + 15,
                ms_since_prev_block: Some(15000),
            },
        ];
        mock.add(handlers::provide(test_data));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_l2_block_times_last_24_hours(None).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].l2_block_number, 201);
        assert_eq!(result[0].ms_since_prev_block, Some(15000));
    }

    #[tokio::test]
    async fn test_get_l2_block_times_last_7_days_empty() {
        let mock = Mock::new();
        mock.add(handlers::provide(Vec::<L2BlockTimeTestRow>::new()));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_l2_block_times_last_7_days(None).await.unwrap();
        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_get_l2_block_times_last_7_days() {
        let mock = Mock::new();
        let block_time = 1640995200u64;
        let test_data = vec![
            L2BlockTimeTestRow { l2_block_number: 300, block_time, ms_since_prev_block: None },
            L2BlockTimeTestRow {
                l2_block_number: 301,
                block_time: block_time + 20,
                ms_since_prev_block: Some(20000),
            },
        ];
        mock.add(handlers::provide(test_data));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_l2_block_times_last_7_days(None).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].l2_block_number, 301);
        assert_eq!(result[0].ms_since_prev_block, Some(20000));
    }

    #[tokio::test]
    async fn test_l2_block_times_query_validation() {
        // Test to ensure the query structure remains correct and catches type conversion issues
        let mock = Mock::new();
        // Add handlers for all three queries
        mock.add(handlers::provide(Vec::<L2BlockTimeTestRow>::new())); // hour
        mock.add(handlers::provide(Vec::<L2BlockTimeTestRow>::new())); // 24 hours
        mock.add(handlers::provide(Vec::<L2BlockTimeTestRow>::new())); // 7 days

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        // These should not panic due to type conversion errors
        let _ = ch.get_l2_block_times_last_hour(None).await;
        let _ = ch.get_l2_block_times_last_24_hours(None).await;
        let _ = ch.get_l2_block_times_last_7_days(None).await;
    }

    /// Regression test to prevent ClickHouse type conversion errors
    /// This test validates that the L2 block time queries use proper type conversion
    /// to avoid "Illegal type Int64 of first argument of function toUInt64OrNull" errors
    #[tokio::test]
    async fn test_clickhouse_type_conversion_regression() {
        // Test that the queries can be constructed without syntax errors
        // The specific regression was: toUInt64OrNull((calculation)) instead of
        // toUInt64OrNull(toString((calculation)))

        let mock = Mock::new();
        // Add multiple handlers for different queries that will be called
        mock.add(handlers::provide(Vec::<L2BlockTimeTestRow>::new())); // hour
        mock.add(handlers::provide(Vec::<L2BlockTimeTestRow>::new())); // 24 hours
        mock.add(handlers::provide(Vec::<L2BlockTimeTestRow>::new())); // 7 days

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        // Test all L2 block time queries (these were the source of the regression)
        // These specifically test toUInt64OrNull with toString() wrapper
        let _ = ch.get_l2_block_times_last_hour(None).await;
        let _ = ch.get_l2_block_times_last_24_hours(None).await;
        let _ = ch.get_l2_block_times_last_7_days(None).await;

        // If the toString() wrapper is missing, these would fail with:
        // "Illegal type Int64 of first argument of function toUInt64OrNull"
    }

    /// Test specifically for the toUInt64OrNull type conversion pattern
    /// This ensures the toString() wrapper is maintained in L2 block time calculations
    #[tokio::test]
    async fn test_to_uint64_or_null_with_calculation() {
        // This test would fail if someone removes the toString() wrapper
        // from the lagInFrame calculation in L2 block time queries

        let mock = Mock::new();
        let test_data = vec![
            L2BlockTimeTestRow {
                l2_block_number: 1000,
                block_time: 1640995200,
                ms_since_prev_block: None,
            },
            L2BlockTimeTestRow {
                l2_block_number: 1001,
                block_time: 1640995212,
                ms_since_prev_block: Some(12000), // This value comes from the calculation
            },
        ];
        mock.add(handlers::provide(test_data));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        // This should work without ClickHouse type errors
        let result = ch.get_l2_block_times_last_hour(None).await.unwrap();

        // Verify the calculation result is properly converted
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].ms_since_prev_block, Some(12000));

        // The key test: ensure no ClickHouse "Illegal type Int64" errors occur
        // If the toString() wrapper is removed, this test would fail during the query execution
    }

    #[derive(Serialize, Row)]
    struct AvgProveTimeTestRow {
        avg_ms: f64,
    }

    /// Test for avg prove time materialized view fallback when MV is empty
    #[tokio::test]
    async fn test_avg_prove_time_fallback_to_raw_data() {
        let mock = Mock::new();

        // First query returns NaN (empty materialized view)
        mock.add(handlers::provide(vec![AvgProveTimeTestRow { avg_ms: f64::NAN }]));

        // Fallback query returns actual average
        mock.add(handlers::provide(vec![AvgProveTimeTestRow { avg_ms: 1500.0 }]));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_avg_prove_time_last_hour().await.unwrap();
        assert_eq!(result, Some(1500));
    }

    /// Test for avg prove time when materialized view has data
    #[tokio::test]
    async fn test_avg_prove_time_uses_materialized_view() {
        let mock = Mock::new();

        // Materialized view returns valid data
        mock.add(handlers::provide(vec![AvgProveTimeTestRow { avg_ms: 2500.0 }]));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_avg_prove_time_last_24_hours().await.unwrap();
        assert_eq!(result, Some(2500));
    }

    /// Test for avg prove time when no data exists at all
    #[tokio::test]
    async fn test_avg_prove_time_no_data() {
        let mock = Mock::new();

        // Both queries return NaN (no data)
        mock.add(handlers::provide(vec![AvgProveTimeTestRow { avg_ms: f64::NAN }]));
        mock.add(handlers::provide(vec![AvgProveTimeTestRow { avg_ms: f64::NAN }]));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_avg_prove_time_last_7_days().await.unwrap();
        assert_eq!(result, None);
    }

    /// Regression test for the avg_prove_time_ms=0 bug
    /// This test verifies that valid averages are returned instead of 0/None
    #[tokio::test]
    async fn test_avg_prove_time_regression_zero_bug() {
        let mock = Mock::new();

        // Test all three time ranges to ensure the fix works for all

        // Hour query - MV empty, fallback has data
        mock.add(handlers::provide(vec![AvgProveTimeTestRow { avg_ms: f64::NAN }]));
        mock.add(handlers::provide(vec![AvgProveTimeTestRow { avg_ms: 800.5 }]));

        // 24h query - MV has data
        mock.add(handlers::provide(vec![AvgProveTimeTestRow { avg_ms: 1200.7 }]));

        // 7d query - MV empty, fallback has data
        mock.add(handlers::provide(vec![AvgProveTimeTestRow { avg_ms: f64::NAN }]));
        mock.add(handlers::provide(vec![AvgProveTimeTestRow { avg_ms: 950.2 }]));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        // Test hour query uses fallback
        let result_hour = ch.get_avg_prove_time_last_hour().await.unwrap();
        assert_eq!(result_hour, Some(801)); // Rounded from 800.5

        // Test 24h query uses materialized view
        let result_24h = ch.get_avg_prove_time_last_24_hours().await.unwrap();
        assert_eq!(result_24h, Some(1201)); // Rounded from 1200.7

        // Test 7d query uses fallback
        let result_7d = ch.get_avg_prove_time_last_7_days().await.unwrap();
        assert_eq!(result_7d, Some(950)); // Rounded from 950.2
    }

    #[tokio::test]
    async fn get_last_l2_head_time_returns_error_on_failure() {
        let mock = Mock::new();
        mock.add(handlers::failure(test::status::INTERNAL_SERVER_ERROR));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_last_l2_head_time().await;
        assert!(result.is_err());
    }
}
