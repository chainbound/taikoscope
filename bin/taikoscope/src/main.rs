//! Entrypoint.

use extractor::Extractor;
use inserter::ClickhouseClient;
use tokio_stream::StreamExt;

use clap::Parser;
use config::Opts;
use tracing::info;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let opts = Opts::parse();

    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    info!("Initializing ClickHouse client...");
    let clickhouse_client = ClickhouseClient::new(&opts.clickhouse_url)?;
    clickhouse_client.init_db().await?;

    let rpc_url = "wss://eth.merkle.io";
    info!("Initializing extractor...");
    let extractor = Extractor::new(rpc_url).await?;
    let mut block_stream = extractor.get_block_stream().await?;

    info!("Processing blocks...");
    while let Some(block) = block_stream.next().await {
        info!("Processing block: {:?}", block.number);

        // Insert block into ClickHouse
        clickhouse_client.insert_block(&block).await?;
        info!("Inserted block: {:?}", block.number);
    }

    Ok(())
}
