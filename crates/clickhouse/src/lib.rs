//! Taikoscope Inserter

use alloy::primitives::{Address, BlockNumber};
use chainio::ITaikoInbox;
use chrono::{DateTime, LocalResult, TimeZone, Utc};
use clickhouse::{Client, Row};
use derive_more::Debug;
pub use extractor::{L1Header, L2Header};
use eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::info;
use url::Url;

// List of tables managed by migrations
const TABLES: &[&str] = &[
    "l1_head_events",
    "preconf_data",
    "l2_head_events",
    "batches",
    "proved_batches",
    "l2_reorgs",
    "forced_inclusion_processed",
    "verified_batches",
    "slashing_events",
];

/// L1 head event
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq)]
pub struct L1HeadEvent {
    /// L1 block number
    pub l1_block_number: u64,
    /// Block hash
    pub block_hash: [u8; 32],
    /// Slot
    pub slot: u64,
    /// Block timestamp
    pub block_ts: u64,
}

/// Preconf data
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreconfData {
    /// Slot
    pub slot: u64,
    /// Candidates
    pub candidates: Vec<[u8; 20]>,
    /// Current operator
    pub current_operator: Option<[u8; 20]>,
    /// Next operator
    pub next_operator: Option<[u8; 20]>,
}

/// L2 head event
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq)]
pub struct L2HeadEvent {
    /// L2 block number
    pub l2_block_number: u64,
    /// Block hash
    pub block_hash: [u8; 32],
    /// Block timestamp
    pub block_ts: u64,
    /// Sum of gas used in the block
    pub sum_gas_used: u128,
    /// Number of transactions
    pub sum_tx: u32,
    /// Sum of priority fees paid
    pub sum_priority_fee: u128,
    /// Sequencer sequencing the block
    pub sequencer: [u8; 20],
}

/// Batch row
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq)]
pub struct BatchRow {
    /// L1 block number
    pub l1_block_number: u64,
    /// Batch ID
    pub batch_id: u64,
    /// Batch size
    pub batch_size: u16,
    /// Proposer address
    pub proposer_addr: [u8; 20],
    /// Blob count
    pub blob_count: u8,
    /// Blob total bytes
    pub blob_total_bytes: u32,
}

/// Proved batch row
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProvedBatchRow {
    /// L1 block number
    pub l1_block_number: u64,
    /// Batch ID
    pub batch_id: u64,
    /// Verifier address
    pub verifier_addr: [u8; 20],
    /// Parent hash
    pub parent_hash: [u8; 32],
    /// Block hash
    pub block_hash: [u8; 32],
    /// State root
    pub state_root: [u8; 32],
}

/// L2 reorg row
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq)]
pub struct L2ReorgRow {
    /// Block number
    pub l2_block_number: u64,
    /// Depth
    pub depth: u16,
}

impl TryFrom<&ITaikoInbox::BatchProposed> for BatchRow {
    type Error = eyre::Error;

    fn try_from(batch: &ITaikoInbox::BatchProposed) -> Result<Self, Self::Error> {
        let batch_size = u16::try_from(batch.info.blocks.len())?;
        let blob_count = u8::try_from(batch.info.blobHashes.len())?;

        let proposer_addr = batch.meta.proposer.into_array();

        Ok(Self {
            l1_block_number: batch.info.proposedIn,
            batch_id: batch.meta.batchId,
            batch_size,
            proposer_addr,
            blob_count,
            blob_total_bytes: batch.info.blobByteSize,
        })
    }
}

impl TryFrom<(&ITaikoInbox::BatchesProved, u64)> for ProvedBatchRow {
    type Error = eyre::Error;

    fn try_from(input: (&ITaikoInbox::BatchesProved, u64)) -> Result<Self, Self::Error> {
        let (proved, l1_block_number) = input;

        if proved.batchIds.is_empty() || proved.transitions.is_empty() {
            return Err(eyre::eyre!("Empty batch IDs or transitions"));
        }

        // For the example, we're just taking the first transition, but you might want to handle
        // all transitions in a real implementation
        let batch_id = proved.batchIds[0];
        let transition = &proved.transitions[0];
        let verifier_addr = proved.verifier.into_array();

        Ok(Self {
            l1_block_number,
            batch_id,
            verifier_addr,
            parent_hash: *transition.parentHash.as_ref(),
            block_hash: *transition.blockHash.as_ref(),
            state_root: *transition.stateRoot.as_ref(),
        })
    }
}

/// Forced inclusion processed row
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq)]
pub struct ForcedInclusionProcessedRow {
    /// Blob hash
    pub blob_hash: [u8; 32],
}

impl TryFrom<&chainio::taiko::wrapper::ITaikoWrapper::ForcedInclusionProcessed>
    for ForcedInclusionProcessedRow
{
    type Error = eyre::Error;

    fn try_from(
        event: &chainio::taiko::wrapper::ITaikoWrapper::ForcedInclusionProcessed,
    ) -> Result<Self, Self::Error> {
        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(event.blobHash.as_slice());

        Ok(Self { blob_hash: hash_bytes })
    }
}

#[derive(Row, Serialize, Deserialize)]
struct MaxTs {
    block_ts: u64,
}

/// Verified batch row
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerifiedBatchRow {
    /// L1 block number
    pub l1_block_number: u64,
    /// Batch ID
    pub batch_id: u64,
    /// Block hash
    pub block_hash: [u8; 32],
}

impl TryFrom<(&chainio::BatchesVerified, u64)> for VerifiedBatchRow {
    type Error = eyre::Error;

    fn try_from(input: (&chainio::BatchesVerified, u64)) -> Result<Self, Self::Error> {
        let (verified, l1_block_number) = input;

        Ok(Self { l1_block_number, batch_id: verified.batch_id, block_hash: verified.block_hash })
    }
}

/// Slashing event row
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq)]
pub struct SlashingEventRow {
    /// L1 block number where slashing occurred
    pub l1_block_number: u64,
    /// Address of the validator that was slashed
    pub validator_addr: [u8; 20],
}

/// Row representing the time it took for a batch to be proven
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq)]
pub struct BatchProveTimeRow {
    /// Batch ID
    pub batch_id: u64,
    /// Seconds between proposal and proof
    pub seconds_to_prove: u64,
}

/// Row representing the time it took for a batch to be verified
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq)]
pub struct BatchVerifyTimeRow {
    /// Batch ID
    pub batch_id: u64,
    /// Seconds between proof and verification
    pub seconds_to_verify: u64,
}

/// Row representing the block number seen at a given minute
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq)]
pub struct L1BlockTimeRow {
    /// Minute timestamp (unix seconds)
    pub minute: u64,
    /// Highest L1 block number within that minute
    pub block_number: u64,
}

/// Row representing L2 block numbers per minute
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq)]
pub struct L2BlockTimeRow {
    /// Minute timestamp
    pub minute: u64,
    /// Highest block number observed in that minute
    pub block_number: u64,
}

/// Clickhouse client
#[derive(Clone, Debug)]
pub struct ClickhouseClient {
    /// Base client
    #[debug(skip)]
    base: Client,
    /// Database name
    db_name: String,
}

impl ClickhouseClient {
    /// Create a new clickhouse client
    pub fn new(url: Url, db_name: String, username: String, password: String) -> Result<Self> {
        let client = Client::default()
            .with_url(url)
            .with_database(db_name.clone())
            .with_user(username)
            .with_password(password);

        Ok(Self { base: client, db_name })
    }

    /// Drop a table if it exists
    async fn drop_table(&self, table_name: &str) -> Result<()> {
        self.base
            .query(&format!("DROP TABLE IF EXISTS {}.{}", self.db_name, table_name))
            .execute()
            .await
            .wrap_err_with(|| format!("Failed to drop {} table", table_name))
    }

    /// Create database and optionally drop existing tables if reset is true
    pub async fn init_db(&self, reset: bool) -> Result<()> {
        // Create database
        self.base
            .query(&format!("CREATE DATABASE IF NOT EXISTS {}", self.db_name))
            .execute()
            .await?;

        if reset {
            for table in TABLES {
                self.drop_table(table).await?;
            }
            info!(db_name = %self.db_name, "Database reset complete");
        }

        // Init schema
        self.init_schema().await?;

        Ok(())
    }

    /// Initialize database schema using SQL migrations
    pub async fn init_schema(&self) -> Result<()> {
        static INIT_SQL: &str = include_str!("../migrations/001_create_tables.sql");
        for stmt in INIT_SQL.split(';') {
            let stmt = stmt.trim();
            if stmt.is_empty() {
                continue;
            }
            let stmt = stmt.replace("${DB}", &self.db_name);
            self.base.query(&stmt).execute().await?;
        }
        Ok(())
    }

    /// Insert header into `ClickHouse`
    pub async fn insert_l1_header(&self, header: &L1Header) -> Result<()> {
        let client = self.base.clone().with_database(&self.db_name);

        // Convert block hash to [u8, 32]
        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(header.hash.as_slice());

        // Convert data into row format
        let event = L1HeadEvent {
            l1_block_number: header.number,
            block_hash: hash_bytes,
            slot: header.slot,
            block_ts: header.timestamp,
        };

        let mut insert = client.insert("l1_head_events")?;
        insert.write(&event).await?;
        insert.end().await?;

        Ok(())
    }

    /// Insert operator candidates into `ClickHouse`
    pub async fn insert_preconf_data(
        &self,
        slot: u64,
        candidates: Vec<Address>,
        current_operator: Option<Address>,
        next_operator: Option<Address>,
    ) -> Result<()> {
        let client = self.base.clone().with_database(&self.db_name);

        let candidate_array = candidates.into_iter().map(|c| c.into_array()).collect();

        let data = PreconfData {
            slot,
            candidates: candidate_array,
            current_operator: current_operator.map(|c| c.into_array()),
            next_operator: next_operator.map(|c| c.into_array()),
        };

        let mut insert = client.insert("preconf_data")?;
        insert.write(&data).await?;
        insert.end().await?;

        Ok(())
    }

    /// Insert aggregated L2 block stats into `ClickHouse`
    pub async fn insert_l2_header(&self, event: &L2HeadEvent) -> Result<()> {
        let client = self.base.clone().with_database(&self.db_name);
        let mut insert = client.insert("l2_head_events")?;
        insert.write(event).await?;
        insert.end().await?;

        Ok(())
    }

    /// Insert batch into `ClickHouse`
    pub async fn insert_batch(&self, batch: &chainio::ITaikoInbox::BatchProposed) -> Result<()> {
        let client = self.base.clone().with_database(&self.db_name);

        // Convert batch into BatchRow
        let batch_row = BatchRow::try_from(batch)?;

        let mut insert = client.insert("batches")?;
        insert.write(&batch_row).await?;
        insert.end().await?;

        Ok(())
    }

    /// Insert a proved batch into `ClickHouse`
    pub async fn insert_proved_batch(
        &self,
        proved: &chainio::ITaikoInbox::BatchesProved,
        l1_block_number: u64,
    ) -> Result<()> {
        let client = self.base.clone().with_database(&self.db_name);

        // For each batch ID and transition pair, create and insert a ProvedBatchRow
        for (i, batch_id) in proved.batchIds.iter().enumerate() {
            // Skip if we don't have a corresponding transition
            if i >= proved.transitions.len() {
                continue;
            }

            // Create a new proved batch with just one batch ID and transition
            let single_proved = chainio::ITaikoInbox::BatchesProved {
                verifier: proved.verifier,
                batchIds: vec![*batch_id],
                transitions: vec![proved.transitions[i].clone()],
            };

            // Convert to row
            let proved_row = ProvedBatchRow::try_from((&single_proved, l1_block_number))?;

            // Insert into ClickHouse
            let mut insert = client.insert("proved_batches")?;
            insert.write(&proved_row).await?;
            insert.end().await?;
        }

        Ok(())
    }

    /// Insert forced inclusion processed into `ClickHouse`
    pub async fn insert_forced_inclusion(
        &self,
        event: &chainio::taiko::wrapper::ITaikoWrapper::ForcedInclusionProcessed,
    ) -> Result<()> {
        let client = self.base.clone().with_database(&self.db_name);

        // Convert forced inclusion processed into ForcedInclusionRow
        let row = ForcedInclusionProcessedRow::try_from(event)?;
        let mut insert = client.insert("forced_inclusion_processed")?;
        insert.write(&row).await?;
        insert.end().await?;

        Ok(())
    }

    /// Insert L2 reorg into `ClickHouse`
    pub async fn insert_l2_reorg(&self, block_number: BlockNumber, depth: u16) -> Result<()> {
        let client = self.base.clone().with_database(&self.db_name);

        let row = L2ReorgRow { l2_block_number: block_number, depth };

        let mut insert = client.insert("l2_reorgs")?;
        insert.write(&row).await?;
        insert.end().await?;

        Ok(())
    }

    /// Insert verified batch into `ClickHouse`
    pub async fn insert_verified_batch(
        &self,
        verified: &chainio::BatchesVerified,
        l1_block_number: u64,
    ) -> Result<()> {
        let client = self.base.clone().with_database(&self.db_name);

        // Convert to row
        let verified_row = VerifiedBatchRow::try_from((verified, l1_block_number))?;

        // Insert into ClickHouse
        let mut insert = client.insert("verified_batches")?;
        insert.write(&verified_row).await?;
        insert.end().await?;

        Ok(())
    }

    /// Get timestamp of the latest L2 head event in UTC.
    pub async fn get_last_l2_head_time(&self) -> Result<Option<DateTime<Utc>>> {
        let client = self.base.clone().with_database(&self.db_name);
        let query =
            format!("SELECT max(block_ts) AS block_ts FROM {}.l2_head_events", &self.db_name);

        // fetch_all so we can detect empty result
        let rows = client
            .query(&query)
            .fetch_all::<MaxTs>()
            .await
            .context("fetching max(block_ts) failed")?;

        // no rows → no data
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };

        // treat epoch-zero (0) as no data
        if row.block_ts == 0 {
            return Ok(None);
        }

        // convert ts → DateTime or None if out-of-range/null
        let ts_opt = match Utc.timestamp_opt(row.block_ts as i64, 0) {
            LocalResult::Single(dt) => Some(dt),
            _ => None,
        };

        Ok(ts_opt)
    }

    /// Get timestamp of the latest L1 head event in UTC.
    pub async fn get_last_l1_head_time(&self) -> Result<Option<DateTime<Utc>>> {
        let client = self.base.clone().with_database(&self.db_name);
        let query =
            format!("SELECT max(block_ts) AS block_ts FROM {}.l1_head_events", &self.db_name);

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

    /// Get timestamp of the latest `BatchProposed` event insertion in UTC.
    pub async fn get_last_batch_time(&self) -> Result<Option<DateTime<Utc>>> {
        let client = self.base.clone().with_database(&self.db_name);
        let query =
            format!("SELECT toUInt64(max(inserted_at)) AS block_ts FROM {}.batches", &self.db_name);

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

    /// Get timestamp of the latest `BatchesVerified` event insertion in UTC.
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

    /// Get all batches that have not been proven and are older than the given cutoff time.
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

    /// Get all proved batch IDs from the `proved_batches` table.
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

    /// Get all batches that have not been verified and are older than the given cutoff time.
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

    /// Get all verified batch IDs from the `verified_batches` table.
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

    /// Get all slashing events that occurred after the given cutoff time.
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

    /// Get all forced inclusion events that occurred after the given cutoff time.
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

    /// Get all L2 reorg events that occurred after the given cutoff time.
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

    /// Get all active gateway addresses observed since the given cutoff time.
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

    /// Get the average time in milliseconds it takes for a batch to be proven
    /// for proofs submitted within the last hour.
    pub async fn get_avg_prove_time_last_hour(&self) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct AvgRow {
            avg_ms: f64,
        }

        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT COALESCE(avg(toUnixTimestamp64Milli(p.inserted_at) - \
                    toUnixTimestamp64Milli(b.inserted_at)), 0) AS avg_ms \
             FROM {db}.proved_batches p \
             INNER JOIN {db}.batches b \
             ON p.l1_block_number = b.l1_block_number AND p.batch_id = b.batch_id \
             WHERE p.inserted_at >= now64() - INTERVAL 1 HOUR",
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
    /// for proofs submitted within the last 24 hours.
    pub async fn get_avg_prove_time_last_24_hours(&self) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct AvgRow {
            avg_ms: f64,
        }

        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT COALESCE(avg(toUnixTimestamp64Milli(p.inserted_at) - \
                    toUnixTimestamp64Milli(b.inserted_at)), 0) AS avg_ms \
             FROM {db}.proved_batches p \
             INNER JOIN {db}.batches b \
             ON p.l1_block_number = b.l1_block_number AND p.batch_id = b.batch_id \
             WHERE p.inserted_at >= now64() - INTERVAL 24 HOUR",
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
    /// for verifications submitted within the last hour.
    pub async fn get_avg_verify_time_last_hour(&self) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct AvgRow {
            avg_ms: f64,
        }

        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT COALESCE(avg(toUnixTimestamp64Milli(v.inserted_at) - \
                    toUnixTimestamp64Milli(p.inserted_at)), 0) AS avg_ms \
             FROM {db}.verified_batches v \
             INNER JOIN {db}.proved_batches p \
             ON v.l1_block_number = p.l1_block_number AND v.batch_id = p.batch_id \
             WHERE v.inserted_at >= now64() - INTERVAL 1 HOUR",
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
    /// for verifications submitted within the last 24 hours.
    pub async fn get_avg_verify_time_last_24_hours(&self) -> Result<Option<u64>> {
        #[derive(Row, Deserialize)]
        struct AvgRow {
            avg_ms: f64,
        }

        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT COALESCE(avg(toUnixTimestamp64Milli(v.inserted_at) - \
                    toUnixTimestamp64Milli(p.inserted_at)), 0) AS avg_ms \
             FROM {db}.verified_batches v \
             INNER JOIN {db}.proved_batches p \
             ON v.l1_block_number = p.l1_block_number AND v.batch_id = p.batch_id \
             WHERE v.inserted_at >= now64() - INTERVAL 24 HOUR",
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
    /// observed within the last hour.
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
    /// observed within the last 24 hours.
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

    /// Get the average interval in milliseconds between consecutive batch
    /// proposals observed within the last hour.
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
    /// proposals observed within the last 24 hours.
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

    /// Get prove times in seconds for batches proved within the last hour.
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

    /// Get prove times in seconds for batches proved within the last 24 hours.
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

    /// Get verify times in seconds for batches verified within the last hour.
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

    /// Get verify times in seconds for batches verified within the last 24 hours.
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

    /// Get L1 block numbers grouped by minute for the last hour.
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

    /// Get L1 block numbers grouped by minute for the last 24 hours.
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

    /// Get max L2 block number for each minute in the last hour.
    pub async fn get_l2_block_times_last_hour(&self) -> Result<Vec<L2BlockTimeRow>> {
        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT intDiv(block_ts, 60) * 60000 AS minute, \
                    max(l2_block_number) AS block_number \
             FROM {db}.l2_head_events \
             WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL 1 HOUR) \
             GROUP BY minute \
             ORDER BY minute",
            db = self.db_name
        );

        let rows = client.query(&query).fetch_all::<L2BlockTimeRow>().await?;
        Ok(rows)
    }

    /// Get max L2 block number for each minute in the last 24 hours.
    pub async fn get_l2_block_times_last_24_hours(&self) -> Result<Vec<L2BlockTimeRow>> {
        let client = self.base.clone().with_database(&self.db_name);
        let query = format!(
            "SELECT intDiv(block_ts, 60) * 60000 AS minute, \
                    max(l2_block_number) AS block_number \
             FROM {db}.l2_head_events \
             WHERE block_ts >= toUnixTimestamp(now64() - INTERVAL 24 HOUR) \
             GROUP BY minute \
             ORDER BY minute",
            db = self.db_name
        );

        let rows = client.query(&query).fetch_all::<L2BlockTimeRow>().await?;
        Ok(rows)
    }

    /// Get the average number of L2 transactions per second for the last hour.
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

    /// Get the average number of L2 transactions per second for the last 24 hours.
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
}

#[cfg(test)]
mod tests {
    use alloy::primitives::FixedBytes;
    use chrono::{TimeZone, Utc};
    use clickhouse::{
        Row,
        test::{Mock, handlers},
    };
    use extractor::L1Header;
    use serde::{Deserialize, Serialize};
    use url::Url;

    use crate::{
        BatchProveTimeRow, BatchRow, BatchVerifyTimeRow, ClickhouseClient,
        ForcedInclusionProcessedRow, L1BlockTimeRow, L1HeadEvent, L2BlockTimeRow, L2HeadEvent,
        L2ReorgRow, PreconfData, ProvedBatchRow, VerifiedBatchRow,
    };

    #[derive(Serialize, Row)]
    struct MaxRow {
        block_ts: u64,
    }

    #[derive(Serialize, Row)]
    struct BatchInfo {
        l1_block_number: u64,
        batch_id: u64,
        inserted_at: u64, // Representing toUnixTimestamp64Milli
    }

    #[derive(Serialize, Row, Deserialize, Debug, PartialEq)]
    struct VerifiedBatchIdRowTest {
        batch_id: u64,
    }

    #[tokio::test]
    async fn test_insert_l1_block() {
        // 1) Spin up mock server
        let mock = Mock::new();

        // 2) Attach recorder to mock server
        let recorder = mock.add(handlers::record::<L1HeadEvent>());

        // 3) Point client to mock server and do inserts
        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();
        let fake =
            L1Header { number: 1, hash: FixedBytes::from_slice(&[0u8; 32]), slot: 1, timestamp: 1 };
        client.insert_l1_header(&fake).await.unwrap();

        // 4) Collect and assert
        let rows: Vec<L1HeadEvent> = recorder.collect().await;
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0],
            L1HeadEvent {
                l1_block_number: 1,
                block_hash: *FixedBytes::from_slice(&[0u8; 32]),
                slot: 1,
                block_ts: 1,
            }
        );
    }

    #[tokio::test]
    async fn test_null_next_preconf_operator() {
        // 1) Spin up mock server
        let mock = Mock::new();

        // 2) Attach recorder to mock server
        let recorder = mock.add(handlers::record::<PreconfData>());

        // 3) Point client to mock server and do inserts
        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let slot = 42;
        let candidates = vec![];
        let current_operator = None;
        let next_operator = None;

        client
            .insert_preconf_data(slot, candidates.clone(), current_operator, next_operator)
            .await
            .unwrap();

        // 4) Collect and assert
        let rows: Vec<PreconfData> = recorder.collect().await;
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0],
            PreconfData { slot, candidates: vec![], current_operator: None, next_operator: None }
        );
    }

    #[tokio::test]
    async fn test_get_last_l2_head_time_empty() {
        // Spin up mock server
        let mock = Mock::new();
        // Provide no rows
        mock.add(handlers::provide(Vec::<MaxRow>::new()));

        // Initialize client
        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseClient::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        // Call the function under test
        let result = ch.get_last_l2_head_time().await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_get_last_l1_head_time_empty() {
        let mock = Mock::new();
        mock.add(handlers::provide(Vec::<MaxRow>::new()));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseClient::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_last_l1_head_time().await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_get_last_l1_head_time() {
        let mock = Mock::new();
        let expected_ts = 123456;
        mock.add(handlers::provide(vec![MaxRow { block_ts: expected_ts }]));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseClient::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_last_l1_head_time().await.unwrap();
        let expected = Utc.timestamp_opt(expected_ts as i64, 0).single().unwrap();
        assert_eq!(result, Some(expected));
    }

    #[tokio::test]
    async fn test_insert_verified_batch() {
        // Spin up mock server
        let mock = Mock::new();

        // Attach recorder to mock server
        let recorder = mock.add(handlers::record::<VerifiedBatchRow>());

        // Point client to mock server and do inserts
        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let verified = chainio::BatchesVerified { batch_id: 42, block_hash: [1u8; 32] };
        let l1_block_number = 12345;

        client.insert_verified_batch(&verified, l1_block_number).await.unwrap();

        // Collect and assert
        let rows: Vec<VerifiedBatchRow> = recorder.collect().await;
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0],
            VerifiedBatchRow {
                l1_block_number,
                batch_id: verified.batch_id,
                block_hash: verified.block_hash,
            }
        );
    }

    #[tokio::test]
    async fn test_get_unverified_batches_older_than() {
        let mock = Mock::new();
        let now_utc = Utc::now();
        let cutoff_time = now_utc - chrono::Duration::hours(1);
        let expected_batch_id = 123;
        let expected_l1_block = 456;
        let inserted_ts_millis = (cutoff_time - chrono::Duration::minutes(10)).timestamp_millis();

        // The mock handler will simply provide the data. Query correctness is implicitly
        // assumed based on similarity to other tested query functions.
        mock.add(handlers::provide(vec![BatchInfo {
            l1_block_number: expected_l1_block,
            batch_id: expected_batch_id,
            inserted_at: inserted_ts_millis as u64,
        }]));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_unverified_batches_older_than(cutoff_time).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, expected_l1_block);
        assert_eq!(result[0].1, expected_batch_id);
        assert_eq!(result[0].2.timestamp_millis(), inserted_ts_millis);
    }

    #[tokio::test]
    async fn test_get_verified_batch_ids() {
        let mock = Mock::new();
        let expected_ids = vec![10, 20, 30];
        let mock_rows = expected_ids
            .iter()
            .map(|id| VerifiedBatchIdRowTest { batch_id: *id })
            .collect::<Vec<_>>();

        mock.add(handlers::provide(mock_rows));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_verified_batch_ids().await.unwrap();
        assert_eq!(result, expected_ids);
    }

    #[tokio::test]
    async fn test_insert_l2_header() {
        let mock = Mock::new();
        let recorder = mock.add(handlers::record::<L2HeadEvent>());

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let event = L2HeadEvent {
            l2_block_number: 5,
            block_hash: [1u8; 32],
            block_ts: 11,
            sum_gas_used: 42,
            sum_tx: 3,
            sum_priority_fee: 100,
            sequencer: [0x11u8; 20],
        };

        client.insert_l2_header(&event).await.unwrap();

        let rows: Vec<L2HeadEvent> = recorder.collect().await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0], event);
    }

    #[tokio::test]
    async fn test_insert_batch() {
        let mock = Mock::new();
        let recorder = mock.add(handlers::record::<BatchRow>());

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let batch = chainio::ITaikoInbox::BatchProposed {
            info: chainio::ITaikoInbox::BatchInfo {
                proposedIn: 10,
                blocks: vec![Default::default(), Default::default()],
                blobHashes: vec![Default::default(); 3],
                blobByteSize: 100,
                ..Default::default()
            },
            meta: chainio::ITaikoInbox::BatchMetadata {
                proposer: [0x11u8; 20].into(),
                batchId: 42,
                ..Default::default()
            },
            ..Default::default()
        };

        client.insert_batch(&batch).await.unwrap();

        let rows: Vec<BatchRow> = recorder.collect().await;
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0],
            BatchRow {
                l1_block_number: 10,
                batch_id: 42,
                batch_size: 2,
                proposer_addr: [0x11u8; 20],
                blob_count: 3,
                blob_total_bytes: 100,
            }
        );
    }

    #[tokio::test]
    async fn test_insert_proved_batch() {
        let mock = Mock::new();
        let recorder = mock.add(handlers::record::<ProvedBatchRow>());

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let transition = chainio::ITaikoInbox::Transition {
            parentHash: FixedBytes::from_slice(&[1u8; 32]),
            blockHash: FixedBytes::from_slice(&[2u8; 32]),
            stateRoot: FixedBytes::from_slice(&[3u8; 32]),
        };

        let proved = chainio::ITaikoInbox::BatchesProved {
            verifier: [0x22u8; 20].into(),
            batchIds: vec![7],
            transitions: vec![transition],
        };

        let l1_block_number = 100u64;
        client.insert_proved_batch(&proved, l1_block_number).await.unwrap();

        let rows: Vec<ProvedBatchRow> = recorder.collect().await;
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0],
            ProvedBatchRow {
                l1_block_number,
                batch_id: 7,
                verifier_addr: [0x22u8; 20],
                parent_hash: [1u8; 32],
                block_hash: [2u8; 32],
                state_root: [3u8; 32],
            }
        );
    }

    #[tokio::test]
    async fn test_insert_forced_inclusion() {
        let mock = Mock::new();
        let recorder = mock.add(handlers::record::<ForcedInclusionProcessedRow>());

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let event = chainio::taiko::wrapper::ITaikoWrapper::ForcedInclusionProcessed {
            blobHash: FixedBytes::from_slice(&[9u8; 32]),
            feeInGwei: 0,
            createdAtBatchId: 0,
            blobByteOffset: 0,
            blobByteSize: 0,
            blobCreatedIn: 0,
        };

        client.insert_forced_inclusion(&event).await.unwrap();

        let rows: Vec<ForcedInclusionProcessedRow> = recorder.collect().await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].blob_hash, [9u8; 32]);
    }

    #[tokio::test]
    async fn test_get_forced_inclusions_since() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![ForcedInclusionProcessedRow { blob_hash: [1u8; 32] }]));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let since = Utc::now() - chrono::Duration::hours(1);
        let rows = client.get_forced_inclusions_since(since).await.unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].blob_hash, [1u8; 32]);
    }

    #[tokio::test]
    async fn test_insert_l2_reorg() {
        let mock = Mock::new();
        let recorder = mock.add(handlers::record::<L2ReorgRow>());

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        client.insert_l2_reorg(50u64, 3).await.unwrap();

        let rows: Vec<L2ReorgRow> = recorder.collect().await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0], L2ReorgRow { l2_block_number: 50, depth: 3 });
    }

    #[tokio::test]
    async fn test_get_last_l2_head_time() {
        let mock = Mock::new();
        let expected_ts = 42u64;
        mock.add(handlers::provide(vec![MaxRow { block_ts: expected_ts }]));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseClient::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_last_l2_head_time().await.unwrap();
        let expected = Utc.timestamp_opt(expected_ts as i64, 0).single().unwrap();
        assert_eq!(result, Some(expected));
    }

    #[tokio::test]
    async fn test_get_last_batch_time() {
        let mock = Mock::new();
        let expected_ts = 77_000u64; // milliseconds since epoch
        mock.add(handlers::provide(vec![MaxRow { block_ts: expected_ts }]));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseClient::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_last_batch_time().await.unwrap();
        let expected = Utc.timestamp_millis_opt(expected_ts as i64).single().unwrap();
        assert_eq!(result, Some(expected));
    }

    #[tokio::test]
    async fn test_get_last_verified_batch_time() {
        let mock = Mock::new();
        let expected_ts = 99_000u64; // milliseconds since epoch
        mock.add(handlers::provide(vec![MaxRow { block_ts: expected_ts }]));

        let url = Url::parse(mock.url()).unwrap();
        let ch =
            ClickhouseClient::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

        let result = ch.get_last_verified_batch_time().await.unwrap();
        let expected = Utc.timestamp_millis_opt(expected_ts as i64).single().unwrap();
        assert_eq!(result, Some(expected));
    }

    #[tokio::test]
    async fn test_get_unproved_batches_older_than() {
        let mock = Mock::new();
        let now_utc = Utc::now();
        let cutoff_time = now_utc - chrono::Duration::hours(2);
        let expected_batch_id = 321;
        let expected_l1_block = 654;
        let inserted_ts_millis = (cutoff_time - chrono::Duration::minutes(30)).timestamp_millis();

        mock.add(handlers::provide(vec![BatchInfo {
            l1_block_number: expected_l1_block,
            batch_id: expected_batch_id,
            inserted_at: inserted_ts_millis as u64,
        }]));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_unproved_batches_older_than(cutoff_time).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, expected_l1_block);
        assert_eq!(result[0].1, expected_batch_id);
        assert_eq!(result[0].2.timestamp_millis(), inserted_ts_millis);
    }

    #[derive(Serialize, Row, Deserialize, Debug, PartialEq)]
    struct ProvedBatchIdRowTest {
        batch_id: u64,
    }

    #[tokio::test]
    async fn test_get_proved_batch_ids() {
        let mock = Mock::new();
        let expected_ids = vec![5, 6];
        let mock_rows = expected_ids
            .iter()
            .map(|id| ProvedBatchIdRowTest { batch_id: *id })
            .collect::<Vec<_>>();

        mock.add(handlers::provide(mock_rows));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_proved_batch_ids().await.unwrap();
        assert_eq!(result, expected_ids);
    }

    #[derive(Serialize, Row)]
    struct AvgRowTest {
        avg_ms: f64,
    }

    #[tokio::test]
    async fn test_get_avg_prove_time_last_hour() {
        let mock = Mock::new();
        let expected = 1500.0f64;
        mock.add(handlers::provide(vec![AvgRowTest { avg_ms: expected }]));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_avg_prove_time_last_hour().await.unwrap();
        assert_eq!(result, Some(expected.round() as u64));
    }

    #[tokio::test]
    async fn test_get_avg_prove_time_last_24_hours() {
        let mock = Mock::new();
        let expected = 1500.0f64;
        mock.add(handlers::provide(vec![AvgRowTest { avg_ms: expected }]));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_avg_prove_time_last_24_hours().await.unwrap();
        assert_eq!(result, Some(expected.round() as u64));
    }

    #[tokio::test]
    async fn test_get_avg_prove_time_last_hour_empty() {
        let mock = Mock::new();
        mock.add(handlers::provide(Vec::<AvgRowTest>::new()));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_avg_prove_time_last_hour().await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_get_avg_prove_time_last_24_hours_empty() {
        let mock = Mock::new();
        mock.add(handlers::provide(Vec::<AvgRowTest>::new()));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_avg_prove_time_last_24_hours().await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_get_avg_verify_time_last_hour() {
        let mock = Mock::new();
        let expected = 2500.0f64;
        mock.add(handlers::provide(vec![AvgRowTest { avg_ms: expected }]));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_avg_verify_time_last_hour().await.unwrap();
        assert_eq!(result, Some(expected.round() as u64));
    }

    #[tokio::test]
    async fn test_get_avg_verify_time_last_24_hours() {
        let mock = Mock::new();
        let expected = 2500.0f64;
        mock.add(handlers::provide(vec![AvgRowTest { avg_ms: expected }]));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_avg_verify_time_last_24_hours().await.unwrap();
        assert_eq!(result, Some(expected.round() as u64));
    }

    #[tokio::test]
    async fn test_get_avg_verify_time_last_hour_empty() {
        let mock = Mock::new();
        mock.add(handlers::provide(Vec::<AvgRowTest>::new()));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_avg_verify_time_last_hour().await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_get_avg_verify_time_last_24_hours_empty() {
        let mock = Mock::new();
        mock.add(handlers::provide(Vec::<AvgRowTest>::new()));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_avg_verify_time_last_24_hours().await.unwrap();
        assert_eq!(result, None);
    }

    #[derive(Serialize, Row)]
    struct CadenceRowTest {
        min_ts: u64,
        max_ts: u64,
        cnt: u64,
    }

    #[tokio::test]
    async fn test_get_l2_block_cadence_last_hour() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![CadenceRowTest { min_ts: 1_000, max_ts: 4_000, cnt: 4 }]));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_l2_block_cadence_last_hour().await.unwrap();
        assert_eq!(result, Some(1_000));
    }

    #[tokio::test]
    async fn test_get_l2_block_cadence_last_24_hours() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![CadenceRowTest { min_ts: 1_000, max_ts: 4_000, cnt: 4 }]));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_l2_block_cadence_last_24_hours().await.unwrap();
        assert_eq!(result, Some(1_000));
    }

    #[tokio::test]
    async fn test_get_batch_posting_cadence_last_hour() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![CadenceRowTest { min_ts: 2_000, max_ts: 6_000, cnt: 3 }]));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_batch_posting_cadence_last_hour().await.unwrap();
        assert_eq!(result, Some(2_000));
    }

    #[tokio::test]
    async fn test_get_batch_posting_cadence_last_24_hours() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![CadenceRowTest { min_ts: 2_000, max_ts: 6_000, cnt: 3 }]));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_batch_posting_cadence_last_24_hours().await.unwrap();
        assert_eq!(result, Some(2_000));
    }

    #[derive(Serialize, Row, Debug, PartialEq, Eq, Clone)]
    struct ProveTimeRowTest {
        batch_id: u64,
        seconds_to_prove: u64,
    }

    #[tokio::test]
    async fn test_get_prove_times_last_hour() {
        let mock = Mock::new();
        let expected = ProveTimeRowTest { batch_id: 7, seconds_to_prove: 42 };
        mock.add(handlers::provide(vec![expected.clone()]));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_prove_times_last_hour().await.unwrap();
        assert_eq!(result, vec![BatchProveTimeRow { batch_id: 7, seconds_to_prove: 42 }]);
    }

    #[tokio::test]
    async fn test_get_prove_times_last_24_hours() {
        let mock = Mock::new();
        let expected = ProveTimeRowTest { batch_id: 7, seconds_to_prove: 42 };
        mock.add(handlers::provide(vec![expected.clone()]));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_prove_times_last_24_hours().await.unwrap();
        assert_eq!(result, vec![BatchProveTimeRow { batch_id: 7, seconds_to_prove: 42 }]);
    }

    #[derive(Serialize, Row, Debug, PartialEq, Eq, Clone)]
    struct VerifyTimeRowTest {
        batch_id: u64,
        seconds_to_verify: u64,
    }

    #[tokio::test]
    async fn test_get_verify_times_last_hour() {
        let mock = Mock::new();
        let expected = VerifyTimeRowTest { batch_id: 11, seconds_to_verify: 120 };
        mock.add(handlers::provide(vec![expected.clone()]));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_verify_times_last_hour().await.unwrap();
        assert_eq!(result, vec![BatchVerifyTimeRow { batch_id: 11, seconds_to_verify: 120 }]);
    }

    #[tokio::test]
    async fn test_get_verify_times_last_24_hours() {
        let mock = Mock::new();
        let expected = VerifyTimeRowTest { batch_id: 11, seconds_to_verify: 120 };
        mock.add(handlers::provide(vec![expected.clone()]));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_verify_times_last_24_hours().await.unwrap();
        assert_eq!(result, vec![BatchVerifyTimeRow { batch_id: 11, seconds_to_verify: 120 }]);
    }

    #[derive(Serialize, Row, Debug, PartialEq, Eq, Clone)]
    struct BlockTimeRowTest {
        minute: u64,
        block_number: u64,
    }

    #[tokio::test]
    async fn test_get_l1_block_times_last_hour() {
        let mock = Mock::new();
        let expected = BlockTimeRowTest { minute: 1, block_number: 2 };
        mock.add(handlers::provide(vec![expected.clone()]));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_l1_block_times_last_hour().await.unwrap();
        assert_eq!(result, vec![L1BlockTimeRow { minute: 1, block_number: 2 }]);
    }

    #[tokio::test]
    async fn test_get_l2_block_times_last_hour() {
        let mock = Mock::new();
        let expected = BlockTimeRowTest { minute: 0, block_number: 42 };
        mock.add(handlers::provide(vec![expected.clone()]));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_l2_block_times_last_hour().await.unwrap();
        assert_eq!(result, vec![L2BlockTimeRow { minute: 0, block_number: 42 }]);
    }

    #[tokio::test]
    async fn test_get_l2_block_times_last_24_hours() {
        let mock = Mock::new();
        let expected = BlockTimeRowTest { minute: 0, block_number: 42 };
        mock.add(handlers::provide(vec![expected.clone()]));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_l2_block_times_last_24_hours().await.unwrap();
        assert_eq!(result, vec![L2BlockTimeRow { minute: 0, block_number: 42 }]);
    }

    #[derive(Serialize, Row, Debug, PartialEq)]
    struct TpsRowTest {
        min_ts: u64,
        max_ts: u64,
        tx_sum: u64,
    }

    #[tokio::test]
    async fn test_get_avg_l2_tps_last_hour() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![TpsRowTest { min_ts: 10, max_ts: 70, tx_sum: 180 }]));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_avg_l2_tps_last_hour().await.unwrap();
        assert_eq!(result, Some(3.0));
    }

    #[tokio::test]
    async fn test_get_avg_l2_tps_last_24_hours() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![TpsRowTest { min_ts: 100, max_ts: 460, tx_sum: 720 }]));

        let url = Url::parse(mock.url()).unwrap();
        let client = ClickhouseClient::new(
            url,
            "test-db".to_owned(),
            "test_user".to_owned(),
            "test_pass".to_owned(),
        )
        .unwrap();

        let result = client.get_avg_l2_tps_last_24_hours().await.unwrap();
        assert_eq!(result, Some(2.0));
    }
}
