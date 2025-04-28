//! Entrypoint.

use extractor::Extractor;
use inserter::ClickhouseClient;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    println!("Initializing ClickHouse client...");
    let clickhouse_client = ClickhouseClient::new("http://localhost:8123")?;
    clickhouse_client.init_schema().await?;

    let rpc_url = "wss://eth.merkle.io";
    println!("Initializing extractor...");
    let extractor = Extractor::new(rpc_url).await?;
    extractor.process_blocks().await?;

    Ok(())
}
