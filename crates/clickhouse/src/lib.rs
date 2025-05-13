//! Taikoscope Inserter

use alloy::primitives::{Address, BlockNumber};
use chainio::ITaikoInbox;
use chrono::{DateTime, LocalResult, TimeZone, Utc};
use clickhouse::{Client, Row};
use derive_more::Debug;
pub use extractor::{L1Header, L2Header};
use eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};
use tracing::info;
use url::Url;

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

impl TryFrom<&L2Header> for L2HeadEvent {
    type Error = eyre::Error;

    fn try_from(header: &L2Header) -> Result<Self, Self::Error> {
        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(header.hash.as_slice());
        let sequencer = header.beneficiary.into_array();

        Ok(Self {
            l2_block_number: header.number,
            block_hash: hash_bytes,
            block_ts: header.timestamp,
            sum_gas_used: header.gas_used as u128,
            sum_tx: 0,           // TODO: pull receipts and sum (or use RPC batch)
            sum_priority_fee: 0, // TODO: pull receipts and sum (or use RPC batch)
            sequencer,
        })
    }
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
        hash_bytes.copy_from_slice(event.forcedInclusion.blobHash.as_slice());

        Ok(Self { blob_hash: hash_bytes })
    }
}

#[derive(Row, Serialize, Deserialize)]
struct MaxTs {
    block_ts: u64,
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
            .with_database(db_name)
            .with_url(url)
            .with_user(username)
            .with_password(password);

        Ok(Self { base: client, db_name: "taikoscope".into() })
    }

    /// Create database and optionally drop existing tables if reset is true
    pub async fn init_db(&self, reset: bool) -> Result<()> {
        // Create database
        self.base
            .query(&format!("CREATE DATABASE IF NOT EXISTS {}", self.db_name))
            .execute()
            .await?;

        const TABLES: &[&str] = &[
            "l1_head_events",
            "preconf_data",
            "l2_head_events",
            "batches",
            "l2_reorgs",
            "forced_inclusion_processed",
        ];

        if reset {
            for table in TABLES {
                self.base
                    .query(&format!("DROP TABLE IF EXISTS {}.{}", self.db_name, table))
                    .execute()
                    .await?;
            }
            info!("Database reset complete for: {}", self.db_name);
        }

        // Init schema
        self.init_schema().await?;

        Ok(())
    }

    /// Init database schema
    pub async fn init_schema(&self) -> Result<()> {
        // Create l1_head_events table
        self.base
            .query(&format!(
                "CREATE TABLE IF NOT EXISTS {}.l1_head_events (
                    l1_block_number UInt64,
                    block_hash FixedString(32),
                    slot UInt64,
                    block_ts UInt64,
                    inserted_at DateTime64(3) DEFAULT now64()
                ) ENGINE = MergeTree()
                ORDER BY (l1_block_number)",
                self.db_name
            ))
            .execute()
            .await
            .wrap_err("Failed to create l1_head_events table")?;

        // Create preconf_data table
        self.base
            .query(&format!(
                "CREATE TABLE IF NOT EXISTS {}.preconf_data (
                    slot UInt64,
                    candidates Array(FixedString(20)),
                    current_operator Nullable(FixedString(20)),
                    next_operator Nullable(FixedString(20)),
                    inserted_at DateTime64(3) DEFAULT now64()
                ) ENGINE = MergeTree()
                ORDER BY (slot)",
                self.db_name
            ))
            .execute()
            .await
            .wrap_err("Failed to create preconf_data table")?;

        // Create l2_head_events table
        self.base
            .query(&format!(
                "CREATE TABLE IF NOT EXISTS {}.l2_head_events (
                    l2_block_number UInt64,
                    block_hash FixedString(32),
                    block_ts UInt64,
                    sum_gas_used UInt128,
                    sum_tx UInt32,
                    sum_priority_fee UInt128,
                    sequencer FixedString(20),
                    inserted_at DateTime64(3) DEFAULT now64()
                ) ENGINE = MergeTree()
                ORDER BY (l2_block_number)",
                self.db_name
            ))
            .execute()
            .await
            .wrap_err("Failed to create l2_head_events table")?;

        // Create batches table
        self.base
            .query(&format!(
                "CREATE TABLE IF NOT EXISTS {}.batches (
                    l1_block_number UInt64,
                    batch_id UInt64,
                    batch_size UInt16,
                    proposer_addr FixedString(20),
                    blob_count UInt8,
                    blob_total_bytes UInt32,
                    inserted_at DateTime64(3) DEFAULT now64()
                ) ENGINE = MergeTree()
                ORDER BY (l1_block_number, batch_id)",
                self.db_name
            ))
            .execute()
            .await
            .wrap_err("Failed to create batches table")?;

        // Create reorgs table
        self.base
            .query(&format!(
                "CREATE TABLE IF NOT EXISTS {}.l2_reorgs (
                    l2_block_number UInt64,
                    depth UInt16,
                    inserted_at DateTime64(3) DEFAULT now64()
                ) ENGINE = MergeTree()
                ORDER BY inserted_at;",
                self.db_name
            ))
            .execute()
            .await
            .wrap_err("Failed to create l2_reorgs table")?;

        // Create forced_inclusion_processed table
        self.base
            .query(&format!(
                "CREATE TABLE IF NOT EXISTS {}.forced_inclusion_processed (
                    blob_hash FixedString(32),
                    inserted_at DateTime64(3) DEFAULT now64()
                ) ENGINE = MergeTree()
                ORDER BY inserted_at;",
                self.db_name
            ))
            .execute()
            .await
            .wrap_err("Failed to create forced_inclusion_processed table")?;

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

    /// Insert L2 header into `ClickHouse`
    pub async fn insert_l2_header(&self, header: &L2Header) -> Result<()> {
        let client = self.base.clone().with_database(&self.db_name);

        let row = L2HeadEvent::try_from(header)?;

        let mut insert = client.insert("l2_head_events")?;
        insert.write(&row).await?;
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

        // convert ts → DateTime or None if out-of-range/null
        let ts_opt = match Utc.timestamp_opt(row.block_ts as i64, 0) {
            LocalResult::Single(dt) => Some(dt),
            _ => None,
        };

        Ok(ts_opt)
    }
}

#[cfg(test)]
mod tests {
    use alloy::primitives::FixedBytes;
    use clickhouse::{
        Row,
        test::{Mock, handlers},
    };
    use extractor::L1Header;
    use serde::Serialize;
    use url::Url;

    use crate::{ClickhouseClient, L1HeadEvent, PreconfData};

    #[derive(Serialize, Row)]
    struct MaxRow {
        block_ts: u64,
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
            "test-db".to_string(),
            "test_user".to_string(),
            "test_pass".to_string(),
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
            "test-db".to_string(),
            "test_user".to_string(),
            "test_pass".to_string(),
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
        let ch = ClickhouseClient::new(url, "test-db".to_string(), "user".into(), "pass".into())
            .unwrap();

        // Call the function under test
        let result = ch.get_last_l2_head_time().await.unwrap();
        assert_eq!(result, None);
    }
}
