//! Entrypoint.
use std::error::Error;

use inserter::ClickhouseClient;

use tokio::runtime::Runtime;

fn main() -> Result<(), Box<dyn Error>> {
    let rt = Runtime::new()?;
    rt.block_on(async_main())
}

async fn async_main() -> Result<(), Box<dyn Error>> {
    println!("Initializing ClickHouse client...");
    let clickhouse_client = ClickhouseClient::new("http://localhost:8123")?;
    clickhouse_client.init_schema().await?;

    println!("Initializing extractor...");
    extractor::extractor();

    Ok(())
}
