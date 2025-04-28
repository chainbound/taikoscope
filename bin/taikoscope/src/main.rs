//! Entrypoint.

use extractor::Extractor;
use inserter::ClickhouseClient;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    println!("Initializing ClickHouse client...");
    let clickhouse_client = ClickhouseClient::new("http://localhost:8123")?;
    clickhouse_client.init_db().await?;

    let rpc_url = "wss://eth.merkle.io";
    println!("Initializing extractor...");
    let extractor = Extractor::new(rpc_url).await?;
    let mut block_stream = extractor.get_block_stream().await?;

    println!("Processing blocks...");
    while let Some(block) = block_stream.next().await {
        println!("Processing block: {:?}", block.number);

        // Insert block into ClickHouse
        clickhouse_client.insert_block(&block).await?;
        println!("Inserted block: {:?}", block.number);
    }

    Ok(())
}
