//! Taikoscope Inserter

use derive_more::Debug;
use eyre::{Result, WrapErr};
use std::sync::Arc;

use clickhouse::Client;
pub use extractor::Block;

/// Clickhouse client
#[derive(Debug)]
pub struct ClickhouseClient {
    #[debug(skip)]
    client: Arc<Client>,
}

impl ClickhouseClient {
    /// Create a new clickhouse client
    pub fn new(url: &str) -> Result<Self> {
        let client = Client::default().with_url(url).with_database("taikoscope");

        // Wrap client
        let client = Arc::new(client);

        Ok(Self { client })
    }

    /// Init database schema
    pub async fn init_schema(&self) -> Result<()> {
        self.client
            .query(
                "CREATE TABLE IF NOT EXISTS l1_head_events (
                l1_block_number UInt64,
                block_hash FixedString(32),
                slot UInt64,
                block_ts DateTime64(3), -- ms
                inserted_at DateTime64(3) DEFAULT now64()
            ) ENGINE = MergeTree()
            ORDER BY (l1_block_number)",
            )
            .execute()
            .await
            .wrap_err("Failed to create l1_head_events table")?;

        Ok(())
    }

    /// Insert block into `ClickHouse`
    pub async fn insert_block(&self, _: &Block) -> Result<()> {
        // TODO: implement

        Ok(())
    }
}
