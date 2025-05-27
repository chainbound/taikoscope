//! `ClickHouse` reader functionality for API
//! Handles read-only operations and analytics queries

use chrono::{DateTime, LocalResult, TimeZone, Utc};
use clickhouse::{Client, Row};
use derive_more::Debug;
use eyre::{Context, Result};
use serde::Deserialize;
use url::Url;

use crate::models::{
    BatchProveTimeRow, BatchVerifyTimeRow, ForcedInclusionProcessedRow, L1BlockTimeRow,
    L2BlockTimeRow, L2ReorgRow, SequencerDistributionRow, SlashingEventRow,
};

#[derive(Row, Deserialize)]
struct MaxTs {
    block_ts: u64,
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

    /// Get last L2 head time
    pub async fn get_last_l2_head_time(&self) -> Result<Option<DateTime<Utc>>> {
        let client = self.base.clone().with_database(&self.db_name);
        let query =
            format!("SELECT max(block_ts) AS block_ts FROM {}.l2_head_events", self.db_name);
        let rows = client
            .query(&query)
            .fetch_all::<MaxTs>()
            .await
            .context("fetching max(block_ts) failed")?;
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
        let client = self.base.clone().with_database(&self.db_name);
        let query =
            format!("SELECT max(block_ts) AS block_ts FROM {}.l1_head_events", self.db_name);

        let rows = client
            .query(&query)
            .fetch_all::<MaxTs>()
            .await
            .context("fetching max(block_ts) failed")?;

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

        let client = self.base.clone().with_database(&self.db_name);
        let query =
            format!("SELECT max(l2_block_number) AS number FROM {}.l2_head_events", &self.db_name);

        let rows = client.query(&query).fetch_all::<MaxNumber>().await?;
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

        let client = self.base.clone().with_database(&self.db_name);
        let query =
            format!("SELECT max(l1_block_number) AS number FROM {}.l1_head_events", &self.db_name);

        let rows = client.query(&query).fetch_all::<MaxNumber>().await?;
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
        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT max(l1_events.block_ts) AS block_ts
             FROM {db}.batches b
             INNER JOIN {db}.l1_head_events l1_events
               ON b.l1_block_number = l1_events.l1_block_number",
            db = &self.db_name
        );

        let rows = client
            .query(&query)
            .fetch_all::<MaxTs>()
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
        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT toUInt64(max(inserted_at)) AS block_ts FROM {}.verified_batches",
            &self.db_name
        );

        let rows = client
            .query(&query)
            .fetch_all::<MaxTs>()
            .await
            .context("fetching max(inserted_at) failed")?;

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

    /// Get all batches that have not been proven and are older than the given cutoff time
    pub async fn get_unproved_batches_older_than(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<Vec<(u64, u64, DateTime<Utc>)>> {
        let client = self.base.clone().with_database(&self.db_name);
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
        let rows = client
            .query(&query)
            .fetch_all::<(u64, u64, u64)>()
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
        let client = self.base.clone().with_database(&self.db_name);
        let query = format!("SELECT batch_id FROM {}.proved_batches", self.db_name);
        let rows = client.query(&query).fetch_all::<ProvedBatchIdRow>().await?;
        Ok(rows.into_iter().map(|r| r.batch_id).collect())
    }

    /// Get all batches that have not been verified and are older than the given cutoff time
    pub async fn get_unverified_batches_older_than(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<Vec<(u64, u64, DateTime<Utc>)>> {
        let client = self.base.clone().with_database(&self.db_name);
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
        let rows = client
            .query(&query)
            .fetch_all::<(u64, u64, u64)>()
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
        let client = self.base.clone().with_database(&self.db_name);
        let query = format!("SELECT batch_id FROM {}.verified_batches", self.db_name);
        let rows = client.query(&query).fetch_all::<VerifiedBatchIdRow>().await?;
        Ok(rows.into_iter().map(|r| r.batch_id).collect())
    }

    /// Get all slashing events that occurred after the given cutoff time
    pub async fn get_slashing_events_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<SlashingEventRow>> {
        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT l1_block_number, validator_addr FROM {}.slashing_events \
             WHERE inserted_at > toDateTime64({}, 3) \
             ORDER BY inserted_at ASC",
            self.db_name,
            since.timestamp_millis() as f64 / 1000.0,
        );
        let rows = client
            .query(&query)
            .fetch_all::<SlashingEventRow>()
            .await
            .context("fetching slashing events failed")?;
        Ok(rows)
    }

    /// Get all forced inclusion events that occurred after the given cutoff time
    pub async fn get_forced_inclusions_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<ForcedInclusionProcessedRow>> {
        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT blob_hash FROM {}.forced_inclusion_processed \
             WHERE inserted_at > toDateTime64({}, 3) \
             ORDER BY inserted_at ASC",
            self.db_name,
            since.timestamp_millis() as f64 / 1000.0,
        );
        let rows = client
            .query(&query)
            .fetch_all::<ForcedInclusionProcessedRow>()
            .await
            .context("fetching forced inclusion events failed")?;
        Ok(rows)
    }

    /// Get all L2 reorg events that occurred after the given cutoff time
    pub async fn get_l2_reorgs_since(&self, since: DateTime<Utc>) -> Result<Vec<L2ReorgRow>> {
        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT l2_block_number, depth FROM {}.l2_reorgs \
             WHERE inserted_at > toDateTime64({}, 3) \
             ORDER BY inserted_at ASC",
            self.db_name,
            since.timestamp_millis() as f64 / 1000.0,
        );
        let rows = client
            .query(&query)
            .fetch_all::<L2ReorgRow>()
            .await
            .context("fetching reorg events failed")?;
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

        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT candidates, current_operator, next_operator FROM {}.preconf_data \
             WHERE inserted_at > toDateTime64({}, 3)",
            self.db_name,
            since.timestamp_millis() as f64 / 1000.0,
        );
        let rows = client.query(&query).fetch_all::<GatewayRow>().await?;
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
        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT sequencer, count() AS blocks FROM {db}.l2_head_events \
             WHERE inserted_at > toDateTime64({}, 3) \
             GROUP BY sequencer ORDER BY blocks DESC",
            since.timestamp_millis() as f64 / 1000.0,
            db = self.db_name,
        );

        let rows = client.query(&query).fetch_all::<SequencerDistributionRow>().await?;
        Ok(rows)
    }

    /// Get the average time in milliseconds it takes for a batch to be proven
    /// for proofs submitted within the last hour
    pub async fn get_avg_prove_time_last_hour(&self) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct AvgRow {
            avg_ms: f64,
        }

        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT COALESCE(avg(prove_time_ms), 0) AS avg_ms \
             FROM {db}.batch_prove_times_mv \
             WHERE proved_at >= now64() - INTERVAL 1 HOUR",
            db = self.db_name
        );

        let rows = client.query(&query).fetch_all::<AvgRow>().await?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };

        if row.avg_ms == 0.0 { Ok(None) } else { Ok(Some(row.avg_ms.round() as u64)) }
    }

    /// Get the average time in milliseconds it takes for a batch to be proven
    /// for proofs submitted within the last 24 hours
    pub async fn get_avg_prove_time_last_24_hours(&self) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct AvgRow {
            avg_ms: f64,
        }

        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT COALESCE(avg(prove_time_ms), 0) AS avg_ms \
             FROM {db}.batch_prove_times_mv \
             WHERE proved_at >= now64() - INTERVAL 24 HOUR",
            db = self.db_name
        );

        let rows = client.query(&query).fetch_all::<AvgRow>().await?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };

        if row.avg_ms == 0.0 { Ok(None) } else { Ok(Some(row.avg_ms.round() as u64)) }
    }

    /// Get the average time in milliseconds it takes for a batch to be proven
    /// for proofs submitted within the last 7 days
    pub async fn get_avg_prove_time_last_7_days(&self) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct AvgRow {
            avg_ms: f64,
        }

        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT COALESCE(avg(prove_time_ms), 0) AS avg_ms \
             FROM {db}.batch_prove_times_mv \
             WHERE proved_at >= now64() - INTERVAL 7 DAY",
            db = self.db_name
        );

        let rows = client.query(&query).fetch_all::<AvgRow>().await?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };

        if row.avg_ms == 0.0 { Ok(None) } else { Ok(Some(row.avg_ms.round() as u64)) }
    }

    /// Get the average time in milliseconds it takes for a batch to be verified
    /// for verifications submitted within the last hour
    pub async fn get_avg_verify_time_last_hour(&self) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct AvgRow {
            avg_ms: f64,
        }

        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT COALESCE(avg(verify_time_ms), 0) AS avg_ms \
             FROM {db}.batch_verify_times_mv \
             WHERE verified_at >= now64() - INTERVAL 1 HOUR",
            db = self.db_name
        );

        let rows = client.query(&query).fetch_all::<AvgRow>().await?;
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

        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT COALESCE(avg(verify_time_ms), 0) AS avg_ms \
             FROM {db}.batch_verify_times_mv \
             WHERE verified_at >= now64() - INTERVAL 24 HOUR",
            db = self.db_name
        );

        let rows = client.query(&query).fetch_all::<AvgRow>().await?;
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

        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT COALESCE(avg(verify_time_ms), 0) AS avg_ms \
             FROM {db}.batch_verify_times_mv \
             WHERE verified_at >= now64() - INTERVAL 7 DAY",
            db = self.db_name
        );

        let rows = client.query(&query).fetch_all::<AvgRow>().await?;
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };

        if row.avg_ms == 0.0 { Ok(None) } else { Ok(Some(row.avg_ms.round() as u64)) }
    }

    /// Get the average interval in milliseconds between consecutive L2 blocks
    /// observed within the last hour
    pub async fn get_l2_block_cadence_last_hour(&self) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct CadenceRow {
            min_ts: u64,
            max_ts: u64,
            cnt: u64,
        }

        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT toUInt64(min(toUnixTimestamp64Milli(inserted_at))) AS min_ts, \
                    toUInt64(max(toUnixTimestamp64Milli(inserted_at))) AS max_ts, \
                    count() as cnt \
             FROM {db}.l2_head_events \
             WHERE inserted_at >= now64() - INTERVAL 1 HOUR",
            db = self.db_name
        );

        let rows = client.query(&query).fetch_all::<CadenceRow>().await?;
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
    pub async fn get_l2_block_cadence_last_24_hours(&self) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct CadenceRow {
            min_ts: u64,
            max_ts: u64,
            cnt: u64,
        }

        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT toUInt64(min(toUnixTimestamp64Milli(inserted_at))) AS min_ts, \
                    toUInt64(max(toUnixTimestamp64Milli(inserted_at))) AS max_ts, \
                    count() as cnt \
             FROM {db}.l2_head_events \
             WHERE inserted_at >= now64() - INTERVAL 24 HOUR",
            db = self.db_name
        );

        let rows = client.query(&query).fetch_all::<CadenceRow>().await?;
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
    pub async fn get_l2_block_cadence_last_7_days(&self) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct CadenceRow {
            min_ts: u64,
            max_ts: u64,
            cnt: u64,
        }

        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT toUInt64(min(toUnixTimestamp64Milli(inserted_at))) AS min_ts, \
                    toUInt64(max(toUnixTimestamp64Milli(inserted_at))) AS max_ts, \
                    count() as cnt \
             FROM {db}.l2_head_events \
             WHERE inserted_at >= now64() - INTERVAL 7 DAY",
            db = self.db_name
        );

        let rows = client.query(&query).fetch_all::<CadenceRow>().await?;
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

        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT toUInt64(min(toUnixTimestamp64Milli(inserted_at))) AS min_ts, \
                    toUInt64(max(toUnixTimestamp64Milli(inserted_at))) AS max_ts, \
                    count() as cnt \
             FROM {db}.batches \
             WHERE inserted_at >= now64() - INTERVAL 1 HOUR",
            db = self.db_name
        );

        let rows = client.query(&query).fetch_all::<CadenceRow>().await?;
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

        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT toUInt64(min(toUnixTimestamp64Milli(inserted_at))) AS min_ts, \
                    toUInt64(max(toUnixTimestamp64Milli(inserted_at))) AS max_ts, \
                    count() as cnt \
             FROM {db}.batches \
             WHERE inserted_at >= now64() - INTERVAL 24 HOUR",
            db = self.db_name
        );

        let rows = client.query(&query).fetch_all::<CadenceRow>().await?;
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

        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT toUInt64(min(toUnixTimestamp64Milli(inserted_at))) AS min_ts, \
                    toUInt64(max(toUnixTimestamp64Milli(inserted_at))) AS max_ts, \
                    count() as cnt \
             FROM {db}.batches \
             WHERE inserted_at >= now64() - INTERVAL 7 DAY",
            db = self.db_name
        );

        let rows = client.query(&query).fetch_all::<CadenceRow>().await?;
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
        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
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

        let rows = client.query(&query).fetch_all::<BatchProveTimeRow>().await?;
        Ok(rows)
    }

    /// Get prove times in seconds for batches proved within the last 24 hours
    pub async fn get_prove_times_last_24_hours(&self) -> Result<Vec<BatchProveTimeRow>> {
        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
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

        let rows = client.query(&query).fetch_all::<BatchProveTimeRow>().await?;
        Ok(rows)
    }

    /// Get prove times in seconds for batches proved within the last 7 days
    pub async fn get_prove_times_last_7_days(&self) -> Result<Vec<BatchProveTimeRow>> {
        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
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

        let rows = client.query(&query).fetch_all::<BatchProveTimeRow>().await?;
        Ok(rows)
    }

    /// Get verify times in seconds for batches verified within the last hour
    pub async fn get_verify_times_last_hour(&self) -> Result<Vec<BatchVerifyTimeRow>> {
        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
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

        let rows = client.query(&query).fetch_all::<BatchVerifyTimeRow>().await?;
        Ok(rows)
    }

    /// Get verify times in seconds for batches verified within the last 24 hours
    pub async fn get_verify_times_last_24_hours(&self) -> Result<Vec<BatchVerifyTimeRow>> {
        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
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

        let rows = client.query(&query).fetch_all::<BatchVerifyTimeRow>().await?;
        Ok(rows)
    }

    /// Get verify times in seconds for batches verified within the last 7 days
    pub async fn get_verify_times_last_7_days(&self) -> Result<Vec<BatchVerifyTimeRow>> {
        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
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

        let rows = client.query(&query).fetch_all::<BatchVerifyTimeRow>().await?;
        Ok(rows)
    }

    /// Get L1 block numbers grouped by minute for the last hour
    pub async fn get_l1_block_times_last_hour(&self) -> Result<Vec<L1BlockTimeRow>> {
        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT toUInt64(toStartOfMinute(fromUnixTimestamp64Milli(block_ts * 1000))) AS minute, \
                    max(l1_block_number) AS block_number \
             FROM {db}.l1_head_events \
             WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL 1 HOUR) \
             GROUP BY minute \
             ORDER BY minute",
            db = self.db_name
        );

        let rows = client.query(&query).fetch_all::<L1BlockTimeRow>().await?;
        Ok(rows)
    }

    /// Get L1 block numbers grouped by minute for the last 24 hours
    pub async fn get_l1_block_times_last_24_hours(&self) -> Result<Vec<L1BlockTimeRow>> {
        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT toUInt64(toStartOfMinute(fromUnixTimestamp64Milli(block_ts * 1000))) AS minute, \
                    max(l1_block_number) AS block_number \
             FROM {db}.l1_head_events \
             WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL 24 HOUR) \
             GROUP BY minute \
             ORDER BY minute",
            db = self.db_name
        );

        let rows = client.query(&query).fetch_all::<L1BlockTimeRow>().await?;
        Ok(rows)
    }

    /// Get L1 block numbers grouped by minute for the last 7 days
    pub async fn get_l1_block_times_last_7_days(&self) -> Result<Vec<L1BlockTimeRow>> {
        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT toUInt64(toStartOfMinute(fromUnixTimestamp64Milli(block_ts * 1000))) AS minute, \
                    max(l1_block_number) AS block_number \
             FROM {db}.l1_head_events \
             WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL 7 DAY) \
             GROUP BY minute \
             ORDER BY minute",
            db = self.db_name
        );

        let rows = client.query(&query).fetch_all::<L1BlockTimeRow>().await?;
        Ok(rows)
    }

    /// Get the time between consecutive L2 blocks for the last hour
    pub async fn get_l2_block_times_last_hour(&self) -> Result<Vec<L2BlockTimeRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l2_block_number: u64,
            block_time: u64,
            seconds_since_prev_block: Option<u64>,
        }

        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT l2_block_number, \
                    block_ts AS block_time, \
                    toUInt64OrNull(toString(block_ts - lagInFrame(block_ts) OVER (ORDER BY l2_block_number))) \
                        AS seconds_since_prev_block \
             FROM {db}.l2_head_events \
             WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL 1 HOUR) \
             ORDER BY l2_block_number",
            db = self.db_name
        );
        let rows = client.query(&query).fetch_all::<RawRow>().await?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let dt = Utc.timestamp_opt(r.block_time as i64, 0).single()?;
                r.seconds_since_prev_block.map(|secs| L2BlockTimeRow {
                    l2_block_number: r.l2_block_number,
                    block_time: dt,
                    seconds_since_prev_block: Some(secs),
                })
            })
            .collect())
    }

    /// Get the time between consecutive L2 blocks for the last 24 hours
    pub async fn get_l2_block_times_last_24_hours(&self) -> Result<Vec<L2BlockTimeRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l2_block_number: u64,
            block_time: u64,
            seconds_since_prev_block: Option<u64>,
        }

        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT l2_block_number, \
                    block_ts AS block_time, \
                    toUInt64OrNull(toString(block_ts - lagInFrame(block_ts) OVER (ORDER BY l2_block_number))) \
                        AS seconds_since_prev_block \
             FROM {db}.l2_head_events \
             WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL 24 HOUR) \
             ORDER BY l2_block_number",
            db = self.db_name
        );
        let rows = client.query(&query).fetch_all::<RawRow>().await?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let dt = Utc.timestamp_opt(r.block_time as i64, 0).single()?;
                r.seconds_since_prev_block.map(|secs| L2BlockTimeRow {
                    l2_block_number: r.l2_block_number,
                    block_time: dt,
                    seconds_since_prev_block: Some(secs),
                })
            })
            .collect())
    }

    /// Get the time between consecutive L2 blocks for the last 7 days
    pub async fn get_l2_block_times_last_7_days(&self) -> Result<Vec<L2BlockTimeRow>> {
        #[derive(Row, Deserialize)]
        struct RawRow {
            l2_block_number: u64,
            block_time: u64,
            seconds_since_prev_block: Option<u64>,
        }

        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT l2_block_number, \
                    block_ts AS block_time, \
                    toUInt64OrNull(toString(block_ts - lagInFrame(block_ts) OVER (ORDER BY l2_block_number))) \
                        AS seconds_since_prev_block \
             FROM {db}.l2_head_events \
             WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL 7 DAY) \
             ORDER BY l2_block_number",
            db = self.db_name
        );
        let rows = client.query(&query).fetch_all::<RawRow>().await?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let dt = Utc.timestamp_opt(r.block_time as i64, 0).single()?;
                r.seconds_since_prev_block.map(|secs| L2BlockTimeRow {
                    l2_block_number: r.l2_block_number,
                    block_time: dt,
                    seconds_since_prev_block: Some(secs),
                })
            })
            .collect())
    }

    /// Get the average number of L2 transactions per second for the last hour
    pub async fn get_avg_l2_tps_last_hour(&self) -> Result<Option<f64>> {
        #[derive(Row, Deserialize)]
        struct TpsRow {
            min_ts: u64,
            max_ts: u64,
            tx_sum: u64,
        }

        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT toUInt64(min(block_ts)) AS min_ts, \
                    toUInt64(max(block_ts)) AS max_ts, \
                    sum(sum_tx) AS tx_sum \
             FROM {db}.l2_head_events \
             WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL 1 HOUR)",
            db = self.db_name
        );

        let rows = client.query(&query).fetch_all::<TpsRow>().await?;
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
    pub async fn get_avg_l2_tps_last_24_hours(&self) -> Result<Option<f64>> {
        #[derive(Row, Deserialize)]
        struct TpsRow {
            min_ts: u64,
            max_ts: u64,
            tx_sum: u64,
        }

        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT toUInt64(min(block_ts)) AS min_ts, \
                    toUInt64(max(block_ts)) AS max_ts, \
                    sum(sum_tx) AS tx_sum \
             FROM {db}.l2_head_events \
             WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL 24 HOUR)",
            db = self.db_name
        );

        let rows = client.query(&query).fetch_all::<TpsRow>().await?;
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
    pub async fn get_avg_l2_tps_last_7_days(&self) -> Result<Option<f64>> {
        #[derive(Row, Deserialize)]
        struct TpsRow {
            min_ts: u64,
            max_ts: u64,
            tx_sum: u64,
        }

        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT toUInt64(min(block_ts)) AS min_ts, \
                    toUInt64(max(block_ts)) AS max_ts, \
                    sum(sum_tx) AS tx_sum \
             FROM {db}.l2_head_events \
             WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL 7 DAY)",
            db = self.db_name
        );

        let rows = client.query(&query).fetch_all::<TpsRow>().await?;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    use clickhouse::test::{Mock, handlers};
    use serde::Serialize;

    use crate::ClickhouseReader;

    #[derive(Serialize, Row)]
    struct MaxNum {
        number: u64,
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
}
