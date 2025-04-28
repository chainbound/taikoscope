//! Taikoscope Inserter

use std::sync::Arc;

use clickhouse::Client;

/// Clickhouse client
pub struct ClickhouseClient {
    client: Arc<Client>,
}

impl ClickhouseClient {
    /// Create a new clickhouse client
    pub fn new(url: &str) -> Self {
        let client = Client::default().with_url(url).with_database("taikoscope");

        // Wrap client
        let client = Arc::new(client);

        Self { client }
    }
}
