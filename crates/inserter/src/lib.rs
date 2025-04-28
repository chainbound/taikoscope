//! Taikoscope Inserter

use alloy::primitives::BlockHash;
use derive_more::Debug;
use eyre::{Result, WrapErr};
use serde::Serialize;

use clickhouse::{Client, Row};
pub use extractor::Block;

/// L1 head event
#[derive(Debug, Row, Serialize)]
pub struct L1HeadEvent {
    /// L1 block number
    pub l1_block_number: u64,
    /// Block hash
    pub block_hash: BlockHash,
    /// Slot
    pub slot: u64,
    /// Block timestamp
    pub block_ts: u64,
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
        self.base
            .query(&format!(
                "CREATE TABLE IF NOT EXISTS {}.l1_head_events (
                l1_block_number UInt64,
                block_hash FixedString(32),
                slot UInt64,
                block_ts DateTime64(3), -- ms
                inserted_at DateTime64(3) DEFAULT now64()
            ) ENGINE = MergeTree()
            ORDER BY (l1_block_number)",
                self.db_name
            ))
            .execute()
            .await
            .wrap_err("Failed to create l1_head_events table")?;

        Ok(())
    }

    /// Insert block into `ClickHouse`
    pub async fn insert_block(&self, block: &Block) -> Result<()> {
        let client = self.base.clone().with_database(&self.db_name);
        // Convert data into row format
        let event = L1HeadEvent {
            l1_block_number: block.number,
            block_hash: block.hash,
            slot: block.slot,
            block_ts: block.timestamp,
        };

        let mut insert = client.insert("l1_head_events")?;
        insert.write(&event).await?;
        insert.end().await?;

        Ok(())
    }
}
