//! Taikoscope Inserter

use clickhouse::{Client, Row};
use derive_more::Debug;
pub use extractor::Block;
use eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};

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

impl TryFrom<&chainio::ITaikoInbox::BatchProposed> for BatchRow {
    type Error = eyre::Error;

    fn try_from(batch: &chainio::ITaikoInbox::BatchProposed) -> Result<Self, Self::Error> {
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

/// Clickhouse client
#[derive(Debug)]
pub struct ClickhouseClient {
    /// Base client
    #[debug(skip)]
    base: Client,
    /// Database name
    db_name: String,
}

impl ClickhouseClient {
    /// Create a new clickhouse client
    pub fn new(url: &str) -> Result<Self> {
        let client = Client::default().with_url(url);

        Ok(Self { base: client, db_name: "taikoscope".into() })
    }

    /// Create database
    pub async fn init_db(&self) -> Result<()> {
        // Drop the existing table if it exists
        self.base
            .query(&format!("DROP TABLE IF EXISTS {}.l1_head_events", self.db_name))
            .execute()
            .await?;

        self.base
            .query(&format!("DROP TABLE IF EXISTS {}.batches", self.db_name))
            .execute()
            .await?;

        // Create database
        self.base
            .query(&format!("CREATE DATABASE IF NOT EXISTS {}", self.db_name))
            .execute()
            .await?;

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
            ) ENGINE = MergeTree(),
            ORDER BY (l1_block_number, batch_id)",
                self.db_name
            ))
            .execute()
            .await
            .wrap_err("Failed to create batches table")?;

        Ok(())
    }

    /// Insert block into `ClickHouse`
    pub async fn insert_block(&self, block: &Block) -> Result<()> {
        let client = self.base.clone().with_database(&self.db_name);

        // Convert block hash to [u8, 32]
        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(block.hash.as_slice());

        // Convert data into row format
        let event = L1HeadEvent {
            l1_block_number: block.number,
            block_hash: hash_bytes,
            slot: block.slot,
            block_ts: block.timestamp,
        };

        let mut insert = client.insert("l1_head_events")?;
        insert.write(&event).await?;
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
}

#[cfg(test)]
mod tests {
    use alloy::primitives::FixedBytes;
    use clickhouse::test::{Mock, handlers};
    use extractor::Block;

    use crate::{ClickhouseClient, L1HeadEvent};

    #[tokio::test]
    async fn test_insert_block() {
        // 1) Spin up mock server
        let mock = Mock::new();

        // 2) Attach recorder to mock server
        let recorder = mock.add(handlers::record::<L1HeadEvent>());

        // 3) Point client to mock server and do inserts
        let client = ClickhouseClient::new(mock.url()).unwrap();
        let fake =
            Block { number: 1, hash: FixedBytes::from_slice(&[0u8; 32]), slot: 1, timestamp: 1 };
        client.insert_block(&fake).await.unwrap();

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
}
