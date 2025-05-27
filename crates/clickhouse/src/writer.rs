//! `ClickHouse` writer functionality for taikoscope
//! Handles database initialization, migrations, and data insertion

use alloy::primitives::{Address, BlockNumber};
use clickhouse::Client;
use derive_more::Debug;
use eyre::{Context, Result};
use tracing::info;
use url::Url;

use crate::{
    L1Header,
    models::{
        BatchRow, ForcedInclusionProcessedRow, L1HeadEvent, L2HeadEvent, L2ReorgRow, PreconfData,
        ProvedBatchRow, VerifiedBatchRow,
    },
    schema::{TABLE_SCHEMAS, TABLES, TableSchema},
};

/// `ClickHouse` writer client for taikoscope (data insertion and migrations)
#[derive(Clone, Debug)]
pub struct ClickhouseWriter {
    /// Base client
    #[debug(skip)]
    base: Client,
    /// Database name
    db_name: String,
}

impl ClickhouseWriter {
    /// Create a new `ClickHouse` writer client
    pub fn new(url: Url, db_name: String, username: String, password: String) -> Result<Self> {
        let client = Client::default()
            .with_url(url)
            .with_database(db_name.clone())
            .with_user(username)
            .with_password(password);

        Ok(Self { base: client, db_name })
    }

    /// Create a table with the given schema
    async fn create_table(&self, schema: &TableSchema) -> Result<()> {
        let query = format!(
            "CREATE TABLE IF NOT EXISTS {}.{} (
                {}
            ) ENGINE = MergeTree()
            ORDER BY ({})",
            self.db_name, schema.name, schema.columns, schema.order_by
        );

        self.base
            .query(&query)
            .execute()
            .await
            .wrap_err_with(|| format!("Failed to create {} table", schema.name))
    }

    /// Drop a table if it exists
    async fn drop_table(&self, table_name: &str) -> Result<()> {
        self.base
            .query(&format!("DROP TABLE IF EXISTS {}.{}", self.db_name, table_name))
            .execute()
            .await
            .wrap_err_with(|| format!("Failed to drop {} table", table_name))
    }

    /// Initialize database and optionally reset
    pub async fn init_db(&self, reset: bool) -> Result<()> {
        self.init_db_with_migrations(reset, true).await
    }

    /// Initialize database with option to skip migrations (useful for tests)
    pub async fn init_db_with_migrations(&self, reset: bool, run_migrations: bool) -> Result<()> {
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

        if run_migrations {
            self.init_schema().await?;
        }
        Ok(())
    }

    /// Initialize schema
    pub async fn init_schema(&self) -> Result<()> {
        for schema in TABLE_SCHEMAS {
            self.create_table(schema).await?;
        }

        // Apply materialized views migration
        self.apply_materialized_views_migration().await?;

        Ok(())
    }

    /// Apply materialized views migration
    async fn apply_materialized_views_migration(&self) -> Result<()> {
        info!("Applying materialized views migration...");
        static MV_SQL: &str = include_str!("../migrations/002_create_materialized_views.sql");

        let statements: Vec<&str> = MV_SQL.split(';').collect();
        info!(
            statement_count = statements.len(),
            "Found {} SQL statements in migration",
            statements.len()
        );

        for (i, stmt) in statements.iter().enumerate() {
            let stmt = stmt.trim();
            if stmt.is_empty() {
                continue;
            }
            let stmt = stmt.replace("${DB}", &self.db_name);

            info!(
                statement_index = i,
                "Executing migration statement: {}",
                stmt.chars().take(100).collect::<String>()
            );

            self.base.query(&stmt).execute().await.wrap_err_with(|| {
                format!("Failed to execute materialized view migration statement {}: {}", i, stmt)
            })?;

            info!(statement_index = i, "Successfully executed migration statement");
        }

        info!("✅ Materialized views migration completed");
        Ok(())
    }

    /// Insert L1 header
    pub async fn insert_l1_header(&self, header: &L1Header) -> Result<()> {
        let client = self.base.clone().with_database(&self.db_name);
        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(header.hash.as_slice());
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

    /// Insert preconfiguration data
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

    /// Insert L2 header event
    pub async fn insert_l2_header(&self, event: &L2HeadEvent) -> Result<()> {
        let client = self.base.clone().with_database(&self.db_name);
        let mut insert = client.insert("l2_head_events")?;
        insert.write(event).await?;
        insert.end().await?;
        Ok(())
    }

    /// Insert a batch
    pub async fn insert_batch(&self, batch: &chainio::ITaikoInbox::BatchProposed) -> Result<()> {
        let client = self.base.clone().with_database(&self.db_name);
        let batch_row = BatchRow::try_from(batch)?;
        let mut insert = client.insert("batches")?;
        insert.write(&batch_row).await?;
        insert.end().await?;
        Ok(())
    }

    /// Insert proved batches
    pub async fn insert_proved_batch(
        &self,
        proved: &chainio::ITaikoInbox::BatchesProved,
        l1_block_number: u64,
    ) -> Result<()> {
        let client = self.base.clone().with_database(&self.db_name);
        for (i, batch_id) in proved.batchIds.iter().enumerate() {
            if i >= proved.transitions.len() {
                continue;
            }
            let single_proved = chainio::ITaikoInbox::BatchesProved {
                verifier: proved.verifier,
                batchIds: vec![*batch_id],
                transitions: vec![proved.transitions[i].clone()],
            };
            let proved_row = ProvedBatchRow::try_from((&single_proved, l1_block_number))?;
            let mut insert = client.insert("proved_batches")?;
            insert.write(&proved_row).await?;
            insert.end().await?;
        }
        Ok(())
    }

    /// Insert forced inclusion processed row
    pub async fn insert_forced_inclusion(
        &self,
        event: &chainio::taiko::wrapper::ITaikoWrapper::ForcedInclusionProcessed,
    ) -> Result<()> {
        let client = self.base.clone().with_database(&self.db_name);
        let row = ForcedInclusionProcessedRow::try_from(event)?;
        let mut insert = client.insert("forced_inclusion_processed")?;
        insert.write(&row).await?;
        insert.end().await?;
        Ok(())
    }

    /// Insert L2 reorg row
    pub async fn insert_l2_reorg(&self, block_number: BlockNumber, depth: u16) -> Result<()> {
        let client = self.base.clone().with_database(&self.db_name);
        let row = L2ReorgRow { l2_block_number: block_number, depth };
        let mut insert = client.insert("l2_reorgs")?;
        insert.write(&row).await?;
        insert.end().await?;
        Ok(())
    }

    /// Insert verified batch row
    pub async fn insert_verified_batch(
        &self,
        verified: &chainio::BatchesVerified,
        l1_block_number: u64,
    ) -> Result<()> {
        let client = self.base.clone().with_database(&self.db_name);
        let verified_row = VerifiedBatchRow::try_from((verified, l1_block_number))?;
        let mut insert = client.insert("verified_batches")?;
        insert.write(&verified_row).await?;
        insert.end().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use alloy::primitives::B256;
    use clickhouse::test::{Mock, handlers};

    #[tokio::test]
    async fn create_table_generates_correct_query() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record_ddl());
        let url = Url::parse(mock.url()).unwrap();
        let writer =
            ClickhouseWriter::new(url, "db".to_owned(), "user".into(), "pass".into()).unwrap();

        writer.create_table(&TABLE_SCHEMAS[0]).await.unwrap();
        let query = ctl.query().await;
        assert!(query.contains("CREATE TABLE IF NOT EXISTS db.l1_head_events"));
    }

    #[tokio::test]
    async fn insert_l1_header_writes_expected_row() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<L1HeadEvent>());

        let url = Url::parse(mock.url()).unwrap();
        let writer =
            ClickhouseWriter::new(url, "db".to_owned(), "user".into(), "pass".into()).unwrap();

        let header = L1Header { number: 1, hash: B256::repeat_byte(1), slot: 2, timestamp: 42 };

        writer.insert_l1_header(&header).await.unwrap();

        let rows: Vec<L1HeadEvent> = ctl.collect().await;
        let expected =
            L1HeadEvent { l1_block_number: 1, block_hash: [1u8; 32], slot: 2, block_ts: 42 };
        assert_eq!(rows, vec![expected]);
    }

    #[tokio::test]
    async fn insert_preconf_data_writes_expected_row() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<PreconfData>());

        let url = Url::parse(mock.url()).unwrap();
        let writer =
            ClickhouseWriter::new(url, "db".to_owned(), "user".into(), "pass".into()).unwrap();

        let candidates = vec![Address::repeat_byte(1), Address::repeat_byte(2)];
        writer
            .insert_preconf_data(5, candidates.clone(), Some(Address::repeat_byte(3)), None)
            .await
            .unwrap();

        let rows: Vec<PreconfData> = ctl.collect().await;
        let expected = PreconfData {
            slot: 5,
            candidates: vec![
                Address::repeat_byte(1).into_array(),
                Address::repeat_byte(2).into_array(),
            ],
            current_operator: Some(Address::repeat_byte(3).into_array()),
            next_operator: None,
        };
        assert_eq!(rows, vec![expected]);
    }

    #[tokio::test]
    async fn insert_l2_reorg_writes_expected_row() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<L2ReorgRow>());

        let url = Url::parse(mock.url()).unwrap();
        let writer =
            ClickhouseWriter::new(url, "db".to_owned(), "user".into(), "pass".into()).unwrap();

        writer.insert_l2_reorg(10, 3).await.unwrap();

        let rows: Vec<L2ReorgRow> = ctl.collect().await;
        let expected = L2ReorgRow { l2_block_number: 10, depth: 3 };
        assert_eq!(rows, vec![expected]);
    }
}
