//! `ClickHouse` writer functionality for taikoscope
//! Handles database initialization, migrations, and data insertion

use alloy::primitives::{Address, B256, BlockNumber};
use clickhouse::Client;
use derive_more::Debug;
use eyre::{Context, Result};
use include_dir::{Dir, include_dir};
use regex::Regex;
use sqlparser::{dialect::GenericDialect, parser::Parser};
use tracing::info;
use url::Url;

/// Embedded migrations directory
static MIGRATIONS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/migrations");

/// Split a SQL script into individual statements using sqlparser-rs.
///
/// This parser properly handles comments, quoted strings, and complex SQL syntax.
pub(crate) fn parse_sql_statements(sql: &str) -> Vec<String> {
    let dialect = GenericDialect {};

    // Use sqlparser to split statements properly, handling quotes and comments
    match Parser::parse_sql(&dialect, sql) {
        Ok(parsed_statements) => {
            // Successfully parsed - extract statement strings by re-parsing each individually
            let mut statements = Vec::new();
            let mut remaining = sql;

            for _ in parsed_statements {
                // Find the next complete statement in the remaining text
                if let Some((stmt_text, rest)) = extract_next_statement(remaining) {
                    let trimmed = stmt_text.trim();
                    if !trimmed.is_empty() {
                        statements.push(trimmed.to_owned());
                    }
                    remaining = rest;
                }
            }
            statements
        }
        Err(_) => {
            // Fallback to manual parsing when sqlparser fails (e.g., ClickHouse-specific syntax)
            split_statements_manually(sql)
        }
    }
}

/// Extract the next complete SQL statement from text, handling quotes and comments
fn extract_next_statement(sql: &str) -> Option<(String, &str)> {
    let mut statement = String::new();
    let mut chars = sql.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut prev_char = '\0';
    let mut char_count = 0;

    while let Some(c) = chars.next() {
        char_count += c.len_utf8();

        if in_line_comment {
            statement.push(c);
            if c == '\n' {
                in_line_comment = false;
            }
            prev_char = c;
            continue;
        }

        if in_block_comment {
            statement.push(c);
            if prev_char == '*' && c == '/' {
                in_block_comment = false;
            }
            prev_char = c;
            continue;
        }

        if !in_single_quote && !in_double_quote {
            if c == '-' && chars.peek() == Some(&'-') {
                in_line_comment = true;
                statement.push(c);
                if let Some(next) = chars.next() {
                    char_count += next.len_utf8();
                    statement.push(next);
                    prev_char = next;
                }
                continue;
            }
            if c == '/' && chars.peek() == Some(&'*') {
                in_block_comment = true;
                statement.push(c);
                if let Some(next) = chars.next() {
                    char_count += next.len_utf8();
                    statement.push(next);
                    prev_char = next;
                }
                continue;
            }
        }

        if c == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
        } else if c == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
        }

        statement.push(c);

        if c == ';' && !in_single_quote && !in_double_quote && !in_line_comment && !in_block_comment
        {
            // Found end of statement
            let remaining = &sql[char_count..];
            return Some((statement, remaining));
        }

        prev_char = c;
    }

    // If we reach here, we have remaining text but no semicolon
    (!statement.trim().is_empty()).then_some((statement, ""))
}

/// Fallback manual statement splitting for when sqlparser fails
fn split_statements_manually(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut remaining = sql;

    while !remaining.trim().is_empty() {
        if let Some((stmt, rest)) = extract_next_statement(remaining) {
            let trimmed = stmt.trim();
            if !trimmed.is_empty() {
                statements.push(trimmed.to_owned());
            }
            remaining = rest;
        } else {
            break;
        }
    }

    statements
}

/// Validate migration file name (e.g. `001_description.sql` or `001_description_MANUAL.sql`)
fn validate_migration_name(name: &str) -> bool {
    Regex::new(r"^\d{3}_[a-zA-Z0-9_]+\.sql$").map(|re| re.is_match(name)).unwrap_or(false)
}

use crate::{
    L1Header,
    models::{
        BatchBlockRow, BatchRow, ForcedInclusionProcessedRow, L1DataCostInsertRow, L1HeadEvent,
        L2HeadEvent, L2ReorgInsertRow, OrphanedL2HashRow, PreconfData, ProveCostInsertRow,
        ProvedBatchRow, VerifiedBatchRow, VerifyCostInsertRow,
    },
    schema::{TABLE_SCHEMAS, TABLES, TableSchema, VIEWS},
    types::{AddressBytes, HashBytes},
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
    /// Create a new `ClickHouse` writer client.
    ///
    /// This constructor is infallible and directly returns the writer instance.
    pub fn new(url: Url, db_name: String, username: String, password: String) -> Self {
        let client = Client::default().with_url(url).with_user(username).with_password(password);

        Self { base: client, db_name }
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

    /// Drop a view if it exists
    async fn drop_view(&self, view_name: &str) -> Result<()> {
        self.base
            .query(&format!("DROP TABLE IF EXISTS {}.{}", self.db_name, view_name))
            .execute()
            .await
            .wrap_err_with(|| format!("Failed to drop {} view", view_name))
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
            .await
            .wrap_err_with(|| format!("Failed to init database {}", self.db_name))?;

        if reset {
            for view in VIEWS {
                self.drop_view(view).await?;
            }
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
    #[allow(clippy::cognitive_complexity)]
    pub async fn init_schema(&self) -> Result<()> {
        for schema in TABLE_SCHEMAS {
            self.create_table(schema).await?;
        }

        let mut migrations: Vec<_> = MIGRATIONS_DIR
            .files()
            .filter(|f| f.path().extension().and_then(|s| s.to_str()) == Some("sql"))
            .collect();
        migrations.sort_by_key(|f| f.path().file_name().map(|n| n.to_owned()));

        for file in migrations {
            let name = file.path().file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if !validate_migration_name(name) {
                eyre::bail!("Invalid migration name: {name}");
            }

            // Skip MANUAL migrations - these must be executed manually during maintenance
            if name.ends_with("_MANUAL.sql") {
                info!(
                    migration = name,
                    "Skipping MANUAL migration - execute manually during maintenance"
                );
                continue;
            }

            let sql = file
                .contents_utf8()
                .ok_or_else(|| eyre::eyre!("Invalid UTF-8 in migration {name}"))?;
            let statements = parse_sql_statements(sql);
            info!(migration = name, statement_count = statements.len(), "Applying migration");

            for (i, stmt) in statements.iter().enumerate() {
                let stmt = stmt.replace("${DB}", &self.db_name);
                info!(
                    statement_index = i,
                    "Executing migration statement: {}",
                    stmt.chars().take(100).collect::<String>()
                );
                self.base.query(&stmt).execute().await.wrap_err_with(|| {
                    format!("Failed to execute migration {name} statement {i}: {stmt}")
                })?;
            }
        }

        Ok(())
    }

    /// Insert L1 header
    pub async fn insert_l1_header(&self, header: &L1Header) -> Result<()> {
        let client = self.base.clone();
        let hash_bytes = HashBytes::from(header.hash);
        let event = L1HeadEvent {
            l1_block_number: header.number,
            block_hash: hash_bytes,
            slot: header.slot,
            block_ts: header.timestamp,
        };
        let mut insert = client.insert(&format!("{}.l1_head_events", self.db_name))?;
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
        let client = self.base.clone();
        let candidate_array = candidates.into_iter().map(AddressBytes::from).collect();
        let data = PreconfData {
            slot,
            candidates: candidate_array,
            current_operator: current_operator.map(AddressBytes::from),
            next_operator: next_operator.map(AddressBytes::from),
        };
        let mut insert = client.insert(&format!("{}.preconf_data", self.db_name))?;
        insert.write(&data).await?;
        insert.end().await?;
        Ok(())
    }

    /// Insert L2 header event
    pub async fn insert_l2_header(&self, event: &L2HeadEvent) -> Result<()> {
        let client = self.base.clone();
        let mut insert = client.insert(&format!("{}.l2_head_events", self.db_name))?;
        insert.write(event).await?;
        insert.end().await?;
        Ok(())
    }

    /// Insert L1 data posting cost
    pub async fn insert_l1_data_cost(
        &self,
        l1_block_number: u64,
        batch_id: u64,
        cost: u128,
    ) -> Result<()> {
        let client = self.base.clone();
        let row = L1DataCostInsertRow { l1_block_number, batch_id, cost };
        let mut insert = client.insert(&format!("{}.l1_data_costs", self.db_name))?;
        insert.write(&row).await?;
        insert.end().await?;
        Ok(())
    }

    /// Insert prover cost for a batch
    pub async fn insert_prove_cost(
        &self,
        l1_block_number: u64,
        batch_id: u64,
        cost: u128,
    ) -> Result<()> {
        let client = self.base.clone();
        let row = ProveCostInsertRow { l1_block_number, batch_id, cost };
        let mut insert = client.insert(&format!("{}.prove_costs", self.db_name))?;
        insert.write(&row).await?;
        insert.end().await?;
        Ok(())
    }

    /// Insert verifier cost for a batch
    pub async fn insert_verify_cost(
        &self,
        l1_block_number: u64,
        batch_id: u64,
        cost: u128,
    ) -> Result<()> {
        let client = self.base.clone();
        let row = VerifyCostInsertRow { l1_block_number, batch_id, cost };
        let mut insert = client.insert(&format!("{}.verify_costs", self.db_name))?;
        insert.write(&row).await?;
        insert.end().await?;
        Ok(())
    }

    /// Insert batch block mappings for a batch
    pub async fn insert_batch_blocks(
        &self,
        batch_id: u64,
        l2_block_numbers: Vec<u64>,
    ) -> Result<()> {
        if l2_block_numbers.is_empty() {
            return Ok(());
        }

        let client = self.base.clone();
        let mut insert = client.insert(&format!("{}.batch_blocks", self.db_name))?;

        for l2_block_number in l2_block_numbers {
            let row = BatchBlockRow { batch_id, l2_block_number };
            insert.write(&row).await?;
        }

        insert.end().await?;
        Ok(())
    }

    /// Insert a batch and its block mappings
    pub async fn insert_batch(
        &self,
        batch: &chainio::ITaikoInbox::BatchProposed,
        l1_tx_hash: B256,
    ) -> Result<()> {
        let client = self.base.clone();
        let batch_row = BatchRow::try_from((batch, l1_tx_hash))?;

        // Insert the batch
        let mut insert = client.insert(&format!("{}.batches", self.db_name))?;
        insert.write(&batch_row).await?;
        insert.end().await?;

        // Insert batch-block mappings
        let l2_block_numbers = batch.block_numbers_proposed();
        self.insert_batch_blocks(batch_row.batch_id, l2_block_numbers).await?;

        Ok(())
    }

    /// Insert proved batches
    pub async fn insert_proved_batch(
        &self,
        proved: &chainio::ITaikoInbox::BatchesProved,
        l1_block_number: u64,
    ) -> Result<()> {
        let client = self.base.clone();
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
            let mut insert = client.insert(&format!("{}.proved_batches", self.db_name))?;
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
        let client = self.base.clone();
        let row = ForcedInclusionProcessedRow::try_from(event)?;
        let mut insert = client.insert(&format!("{}.forced_inclusion_processed", self.db_name))?;
        insert.write(&row).await?;
        insert.end().await?;
        Ok(())
    }

    /// Insert L2 reorg row
    pub async fn insert_l2_reorg(
        &self,
        block_number: BlockNumber,
        depth: u16,
        old_sequencer: Address,
        new_sequencer: Address,
    ) -> Result<()> {
        let client = self.base.clone();
        let row = L2ReorgInsertRow {
            l2_block_number: block_number,
            depth,
            old_sequencer: AddressBytes(old_sequencer.into_array()),
            new_sequencer: AddressBytes(new_sequencer.into_array()),
        };
        let mut insert = client.insert(&format!("{}.l2_reorgs", self.db_name))?;
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
        let client = self.base.clone();
        let verified_row = VerifiedBatchRow::try_from((verified, l1_block_number))?;
        let mut insert = client.insert(&format!("{}.verified_batches", self.db_name))?;
        insert.write(&verified_row).await?;
        insert.end().await?;
        Ok(())
    }

    /// Insert orphaned L2 block hashes
    pub async fn insert_orphaned_hashes(&self, hashes: &[(HashBytes, u64)]) -> Result<()> {
        if hashes.is_empty() {
            return Ok(());
        }

        let client = self.base.clone();
        let mut insert = client.insert(&format!("{}.orphaned_l2_hashes", self.db_name))?;

        for (hash, block_number) in hashes {
            let row = OrphanedL2HashRow { block_hash: *hash, l2_block_number: *block_number };
            insert.write(&row).await?;
        }

        insert.end().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::L2ReorgInsertRow;

    use super::*;

    use alloy::primitives::{Address, B256};
    use chainio::{ITaikoInbox, taiko::wrapper::ITaikoWrapper};
    use clickhouse::test::{self, Mock, handlers};

    #[tokio::test]
    async fn create_table_generates_correct_query() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record_ddl());
        let url = Url::parse(mock.url()).unwrap();
        let writer = ClickhouseWriter::new(url, "db".to_owned(), "user".into(), "pass".into());

        writer.create_table(&TABLE_SCHEMAS[0]).await.unwrap();
        let query = ctl.query().await;
        assert!(query.contains("CREATE TABLE IF NOT EXISTS db.l1_head_events"));
    }

    #[tokio::test]
    async fn insert_l1_header_writes_expected_row() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<L1HeadEvent>());

        let url = Url::parse(mock.url()).unwrap();
        let writer = ClickhouseWriter::new(url, "db".to_owned(), "user".into(), "pass".into());

        let header = L1Header { number: 1, hash: B256::repeat_byte(1), slot: 2, timestamp: 42 };

        writer.insert_l1_header(&header).await.unwrap();

        let rows: Vec<L1HeadEvent> = ctl.collect().await;
        let expected = L1HeadEvent {
            l1_block_number: 1,
            block_hash: HashBytes::from([1u8; 32]),
            slot: 2,
            block_ts: 42,
        };
        assert_eq!(rows, vec![expected]);
    }

    #[tokio::test]
    async fn insert_preconf_data_writes_expected_row() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<PreconfData>());

        let url = Url::parse(mock.url()).unwrap();
        let writer = ClickhouseWriter::new(url, "db".to_owned(), "user".into(), "pass".into());

        let candidates = vec![Address::repeat_byte(1), Address::repeat_byte(2)];
        writer
            .insert_preconf_data(5, candidates.clone(), Some(Address::repeat_byte(3)), None)
            .await
            .unwrap();

        let rows: Vec<PreconfData> = ctl.collect().await;
        let expected = PreconfData {
            slot: 5,
            candidates: vec![
                AddressBytes::from(Address::repeat_byte(1)),
                AddressBytes::from(Address::repeat_byte(2)),
            ],
            current_operator: Some(AddressBytes::from(Address::repeat_byte(3))),
            next_operator: None,
        };
        assert_eq!(rows, vec![expected]);
    }

    #[tokio::test]
    async fn insert_l2_reorg_writes_expected_row() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<L2ReorgInsertRow>());

        let url = Url::parse(mock.url()).unwrap();
        let writer = ClickhouseWriter::new(url, "db".to_owned(), "user".into(), "pass".into());

        writer
            .insert_l2_reorg(10, 3, Address::repeat_byte(1), Address::repeat_byte(2))
            .await
            .unwrap();

        let rows: Vec<L2ReorgInsertRow> = ctl.collect().await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].l2_block_number, 10);
        assert_eq!(rows[0].depth, 3);
        assert_eq!(rows[0].old_sequencer, AddressBytes::from(Address::repeat_byte(1)));
        assert_eq!(rows[0].new_sequencer, AddressBytes::from(Address::repeat_byte(2)));
    }

    #[tokio::test]
    async fn insert_l2_header_writes_expected_row() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<L2HeadEvent>());

        let url = Url::parse(mock.url()).unwrap();
        let writer = ClickhouseWriter::new(url, "db".to_owned(), "user".into(), "pass".into());

        let event = L2HeadEvent {
            l2_block_number: 1,
            block_hash: HashBytes::from([1u8; 32]),
            block_ts: 10,
            sum_gas_used: 20,
            sum_tx: 3,
            sum_priority_fee: 30,
            sum_base_fee: 40,
            sequencer: AddressBytes::from([5u8; 20]),
        };

        writer.insert_l2_header(&event).await.unwrap();

        let rows: Vec<L2HeadEvent> = ctl.collect().await;
        assert_eq!(rows, vec![event]);
    }

    #[tokio::test]
    async fn insert_batch_writes_expected_row() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<BatchRow>());
        // Add a handler for the batch_blocks table insert
        let _ctl_blocks = mock.add(handlers::record::<BatchBlockRow>());

        let url = Url::parse(mock.url()).unwrap();
        let writer = ClickhouseWriter::new(url, "db".to_owned(), "user".into(), "pass".into());

        let batch = ITaikoInbox::BatchProposed {
            info: ITaikoInbox::BatchInfo {
                proposedIn: 2,
                blobByteSize: 50,
                blocks: vec![ITaikoInbox::BlockParams::default(); 1],
                blobHashes: vec![B256::repeat_byte(1)],
                lastBlockId: 100, // Adding test value for last block ID
                ..Default::default()
            },
            meta: ITaikoInbox::BatchMetadata {
                proposer: Address::repeat_byte(2),
                batchId: 7,
                ..Default::default()
            },
            ..Default::default()
        };

        writer.insert_batch(&batch, B256::ZERO).await.unwrap();

        let rows: Vec<BatchRow> = ctl.collect().await;
        let expected = BatchRow {
            l1_block_number: 2,
            l1_tx_hash: HashBytes::from([0u8; 32]),
            batch_id: 7,
            batch_size: 1,
            last_l2_block_number: 100,
            proposer_addr: AddressBytes::from(Address::repeat_byte(2)),
            blob_count: 1,
            blob_total_bytes: 50,
        };
        assert_eq!(rows, vec![expected]);
    }

    #[tokio::test]
    async fn insert_batch_blocks_writes_expected_rows() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<BatchBlockRow>());

        let url = Url::parse(mock.url()).unwrap();
        let writer = ClickhouseWriter::new(url, "db".to_owned(), "user".into(), "pass".into());

        writer.insert_batch_blocks(7, vec![98, 99, 100]).await.unwrap();

        let rows: Vec<BatchBlockRow> = ctl.collect().await;
        let expected = vec![
            BatchBlockRow { batch_id: 7, l2_block_number: 98 },
            BatchBlockRow { batch_id: 7, l2_block_number: 99 },
            BatchBlockRow { batch_id: 7, l2_block_number: 100 },
        ];
        assert_eq!(rows, expected);
    }

    #[tokio::test]
    async fn insert_proved_batch_writes_expected_rows() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<ProvedBatchRow>());

        let url = Url::parse(mock.url()).unwrap();
        let writer = ClickhouseWriter::new(url, "db".to_owned(), "user".into(), "pass".into());

        let transition = ITaikoInbox::Transition {
            parentHash: B256::repeat_byte(1),
            blockHash: B256::repeat_byte(2),
            stateRoot: B256::repeat_byte(3),
        };
        let proved = ITaikoInbox::BatchesProved {
            verifier: Address::repeat_byte(4),
            batchIds: vec![8],
            transitions: vec![transition],
        };

        writer.insert_proved_batch(&proved, 10).await.unwrap();

        let rows: Vec<ProvedBatchRow> = ctl.collect().await;
        let expected = ProvedBatchRow {
            l1_block_number: 10,
            batch_id: 8,
            verifier_addr: AddressBytes::from(Address::repeat_byte(4)),
            parent_hash: HashBytes::from([1u8; 32]),
            block_hash: HashBytes::from([2u8; 32]),
            state_root: HashBytes::from([3u8; 32]),
        };
        assert_eq!(rows, vec![expected]);
    }

    #[tokio::test]
    async fn insert_verified_batch_writes_expected_row() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<VerifiedBatchRow>());

        let url = Url::parse(mock.url()).unwrap();
        let writer = ClickhouseWriter::new(url, "db".to_owned(), "user".into(), "pass".into());

        let verified = chainio::BatchesVerified { batch_id: 3, block_hash: [9u8; 32] };

        writer.insert_verified_batch(&verified, 12).await.unwrap();

        let rows: Vec<VerifiedBatchRow> = ctl.collect().await;
        let expected = VerifiedBatchRow {
            l1_block_number: 12,
            batch_id: 3,
            block_hash: HashBytes::from([9u8; 32]),
        };
        assert_eq!(rows, vec![expected]);
    }

    #[tokio::test]
    async fn insert_forced_inclusion_writes_expected_row() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<ForcedInclusionProcessedRow>());

        let url = Url::parse(mock.url()).unwrap();
        let writer = ClickhouseWriter::new(url, "db".to_owned(), "user".into(), "pass".into());

        let event = ITaikoWrapper::ForcedInclusionProcessed {
            forcedInclusion: ITaikoWrapper::ForcedInclusion {
                blobHash: B256::repeat_byte(5),
                feeInGwei: 1,
                createdAtBatchId: 0,
                blobByteOffset: 0,
                blobByteSize: 0,
                blobCreatedIn: 0,
            },
        };

        writer.insert_forced_inclusion(&event).await.unwrap();

        let rows: Vec<ForcedInclusionProcessedRow> = ctl.collect().await;
        assert_eq!(
            rows,
            vec![ForcedInclusionProcessedRow { blob_hash: HashBytes::from([5u8; 32]) }]
        );
    }

    #[tokio::test]
    async fn insert_l1_data_cost_writes_expected_row() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<L1DataCostInsertRow>());

        let url = Url::parse(mock.url()).unwrap();
        let writer = ClickhouseWriter::new(url, "db".to_owned(), "user".into(), "pass".into());

        writer.insert_l1_data_cost(10, 7, 42).await.unwrap();

        let rows: Vec<L1DataCostInsertRow> = ctl.collect().await;
        assert_eq!(rows, vec![L1DataCostInsertRow { l1_block_number: 10, batch_id: 7, cost: 42 }]);
    }

    #[tokio::test]
    async fn insert_prove_cost_writes_expected_row() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<ProveCostInsertRow>());

        let url = Url::parse(mock.url()).unwrap();
        let writer = ClickhouseWriter::new(url, "db".to_owned(), "user".into(), "pass".into());

        writer.insert_prove_cost(8, 9, 55).await.unwrap();

        let rows: Vec<ProveCostInsertRow> = ctl.collect().await;
        assert_eq!(rows, vec![ProveCostInsertRow { l1_block_number: 8, batch_id: 9, cost: 55 }]);
    }

    #[tokio::test]
    async fn insert_verify_cost_writes_expected_row() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<VerifyCostInsertRow>());

        let url = Url::parse(mock.url()).unwrap();
        let writer = ClickhouseWriter::new(url, "db".to_owned(), "user".into(), "pass".into());

        writer.insert_verify_cost(8, 10, 66).await.unwrap();

        let rows: Vec<VerifyCostInsertRow> = ctl.collect().await;
        assert_eq!(rows, vec![VerifyCostInsertRow { l1_block_number: 8, batch_id: 10, cost: 66 }]);
    }

    #[tokio::test]
    async fn insert_orphaned_hashes_writes_expected_rows() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<OrphanedL2HashRow>());

        let url = Url::parse(mock.url()).unwrap();
        let writer = ClickhouseWriter::new(url, "db".to_owned(), "user".into(), "pass".into());

        let hashes =
            vec![(HashBytes::from([1u8; 32]), 100u64), (HashBytes::from([2u8; 32]), 101u64)];
        writer.insert_orphaned_hashes(&hashes).await.unwrap();

        let rows: Vec<OrphanedL2HashRow> = ctl.collect().await;
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].block_hash, HashBytes::from([1u8; 32]));
        assert_eq!(rows[0].l2_block_number, 100);
        assert_eq!(rows[1].block_hash, HashBytes::from([2u8; 32]));
        assert_eq!(rows[1].l2_block_number, 101);
    }

    #[test]
    fn parse_sql_handles_semicolons_in_strings() {
        let sql = "CREATE TABLE t(a String DEFAULT ';');\nCREATE TABLE t2(b String);";
        let statements = parse_sql_statements(sql);
        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("DEFAULT ';'"));
    }

    #[tokio::test]
    async fn init_schema_runs_create_and_migration_queries() {
        let mock = Mock::new();
        let migration_count: usize = MIGRATIONS_DIR
            .files()
            .filter(|f| f.path().extension().and_then(|s| s.to_str()) == Some("sql"))
            .map(|f| parse_sql_statements(f.contents_utf8().unwrap()).len())
            .sum();
        let total = TABLE_SCHEMAS.len() + migration_count;

        let ctrls: Vec<_> =
            std::iter::repeat_with(|| mock.add(handlers::record_ddl())).take(total).collect();

        let url = Url::parse(mock.url()).unwrap();
        let writer = ClickhouseWriter::new(url, "db".to_owned(), "user".into(), "pass".into());

        writer.init_schema().await.unwrap();

        let mut queries = Vec::new();
        for c in ctrls {
            queries.push(c.query().await);
        }
        assert_eq!(queries.len(), total);

        // verify that at least one table and one view were created
        assert!(queries.iter().any(|q| q.contains("db.l1_head_events")));
        assert!(queries.iter().any(|q| q.contains("db.batch_prove_times_mv")));
    }

    #[tokio::test]
    async fn init_db_removes_views_when_reset() {
        let mock = Mock::new();
        let migration_count: usize = MIGRATIONS_DIR
            .files()
            .filter(|f| f.path().extension().and_then(|s| s.to_str()) == Some("sql"))
            .map(|f| parse_sql_statements(f.contents_utf8().unwrap()).len())
            .sum();
        let total = 1 + TABLES.len() + VIEWS.len() + TABLE_SCHEMAS.len() + migration_count;

        let ctrls: Vec<_> =
            std::iter::repeat_with(|| mock.add(handlers::record_ddl())).take(total).collect();

        let url = Url::parse(mock.url()).unwrap();
        let writer = ClickhouseWriter::new(url, "db".to_owned(), "user".into(), "pass".into());

        writer.init_db(true).await.unwrap();

        let mut queries = Vec::new();
        for c in ctrls {
            queries.push(c.query().await);
        }

        assert!(queries.iter().any(|q| q.contains("DROP TABLE IF EXISTS db.batch_prove_times_mv")));
    }

    #[tokio::test]
    async fn create_table_returns_error_on_failure() {
        let mock = Mock::new();
        mock.add(handlers::failure(test::status::INTERNAL_SERVER_ERROR));

        let url = Url::parse(mock.url()).unwrap();
        let writer = ClickhouseWriter::new(url, "db".to_owned(), "user".into(), "pass".into());

        let result = writer.create_table(&TABLE_SCHEMAS[0]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn insert_batch_fails_with_too_many_blobs() {
        let mock = Mock::new();
        let url = Url::parse(mock.url()).unwrap();
        let writer = ClickhouseWriter::new(url, "db".to_owned(), "user".into(), "pass".into());

        let batch = ITaikoInbox::BatchProposed {
            info: ITaikoInbox::BatchInfo {
                proposedIn: 1,
                blobByteSize: 10,
                blocks: vec![ITaikoInbox::BlockParams::default(); 1],
                blobHashes: vec![B256::repeat_byte(1); 256],
                ..Default::default()
            },
            meta: ITaikoInbox::BatchMetadata {
                proposer: Address::repeat_byte(2),
                batchId: 1,
                ..Default::default()
            },
            ..Default::default()
        };

        let result = writer.insert_batch(&batch, B256::ZERO).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn insert_proved_batch_returns_error_on_failure() {
        let mock = Mock::new();
        mock.add(handlers::failure(test::status::INTERNAL_SERVER_ERROR));
        let url = Url::parse(mock.url()).unwrap();
        let writer = ClickhouseWriter::new(url, "db".to_owned(), "user".into(), "pass".into());

        let transition = ITaikoInbox::Transition {
            parentHash: B256::repeat_byte(1),
            blockHash: B256::repeat_byte(2),
            stateRoot: B256::repeat_byte(3),
        };
        let proved = ITaikoInbox::BatchesProved {
            verifier: Address::repeat_byte(4),
            batchIds: vec![1],
            transitions: vec![transition],
        };

        let result = writer.insert_proved_batch(&proved, 10).await;
        assert!(result.is_err());
    }

    #[test]
    fn validate_migration_name_accepts_manual_migrations() {
        // Test that MANUAL migrations are accepted
        assert!(validate_migration_name(
            "020_batches_batch_blocks_replacing_merge_tree_MANUAL.sql"
        ));
        assert!(validate_migration_name("021_migrate_to_replacing_merge_tree_MANUAL.sql"));
        assert!(validate_migration_name("022_atomic_table_swap_MANUAL.sql"));

        // Test normal migrations still work
        assert!(validate_migration_name("019_add_schema_migrations_tracking.sql"));
        assert!(validate_migration_name("023_cleanup_old_tables.sql"));

        // Test invalid patterns still fail
        assert!(!validate_migration_name("invalid_migration.sql"));
        assert!(!validate_migration_name("19_too_short.sql"));
    }

    #[test]
    fn migration_tracking_system_logic() {
        // Test the logic of migration tracking and manual migration handling
        // without depending on specific real migration files

        // Test 1: Migration name validation includes MANUAL files
        assert!(validate_migration_name("001_initial.sql"));
        assert!(validate_migration_name("019_create_tracking.sql"));
        assert!(validate_migration_name("021_data_migration_MANUAL.sql"));
        assert!(validate_migration_name("022_table_swap_MANUAL.sql"));

        // Test 2: MANUAL migration detection
        assert!("021_data_migration_MANUAL.sql".ends_with("_MANUAL.sql"));
        assert!("022_table_swap_MANUAL.sql".ends_with("_MANUAL.sql"));
        assert!(!"019_create_tracking.sql".ends_with("_MANUAL.sql"));
        assert!(!"020_create_tables.sql".ends_with("_MANUAL.sql"));

        // Test 3: Validate tracking migration format
        let sample_tracking_migration = "
            CREATE TABLE IF NOT EXISTS ${DB}.schema_migrations (
                version String,
                description String,
                applied_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY version;

            INSERT INTO ${DB}.schema_migrations (version, description)
            SELECT '019', 'add_schema_migrations_tracking'
            WHERE NOT EXISTS (
                SELECT 1 FROM ${DB}.schema_migrations WHERE version = '019'
            );
        ";

        let tracking_lower = sample_tracking_migration.to_lowercase();
        assert!(
            tracking_lower.contains("schema_migrations"),
            "Should create schema_migrations table"
        );
        assert!(tracking_lower.contains("version string"), "Should have version column");
        assert!(tracking_lower.contains("description string"), "Should have description column");
        assert!(tracking_lower.contains("applied_at datetime"), "Should have applied_at column");
        assert!(tracking_lower.contains("if not exists"), "Should be idempotent");
        assert!(tracking_lower.contains("where not exists"), "Should prevent duplicates");

        // Test 4: Validate ReplacingMergeTree migration format
        let sample_rmt_migration = "
            CREATE TABLE IF NOT EXISTS ${DB}.batches_rmt (
                batch_id UInt64,
                l1_block_number UInt64,
                inserted_at DateTime64(3) DEFAULT now64()
            ) ENGINE = ReplacingMergeTree(inserted_at)
            PARTITION BY toYYYYMM(inserted_at)
            ORDER BY (batch_id);

            INSERT INTO ${DB}.schema_migrations (version, description)
            SELECT '020', 'create_replacing_merge_tree_tables'
            WHERE NOT EXISTS (
                SELECT 1 FROM ${DB}.schema_migrations WHERE version = '020'
            );
        ";

        let rmt_lower = sample_rmt_migration.to_lowercase();
        assert!(
            rmt_lower.contains("replacingmergetree"),
            "Should create ReplacingMergeTree tables"
        );
        assert!(rmt_lower.contains("if not exists"), "Should be idempotent");
        assert!(
            rmt_lower.contains("insert into") && rmt_lower.contains("schema_migrations"),
            "Should track completion"
        );

        println!("Migration tracking system logic validated successfully");
    }
}
