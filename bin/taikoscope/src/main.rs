//! Entrypoint.

use inserter::ClickhouseClient;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    println!("Initializing ClickHouse client...");
    let clickhouse_client = ClickhouseClient::new("http://localhost:8123")?;
    clickhouse_client.init_schema().await?;

    println!("Initializing extractor...");
    extractor::extractor();

    Ok(())
}
