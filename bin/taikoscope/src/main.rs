//! Entrypoint.

use extractor::Extractor;
use inserter::ClickhouseClient;
use tokio_stream::StreamExt;

use alloy_primitives::Address;
use clap::Parser;
use config::Opts;
use std::str::FromStr;
use tracing::info;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let opts = Opts::parse();

    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    info!("Initializing ClickHouse client...");
    let clickhouse_client = ClickhouseClient::new(&opts.clickhouse_url)?;
    clickhouse_client.init_db().await?;

    info!("Initializing extractor...");
    let inbox_address = Address::from_str(&opts.inbox_address).unwrap();
    let extractor = Extractor::new(&opts.l1_rpc_url, &opts.l2_rpc_url, inbox_address).await?;

    let mut block_stream = extractor.get_block_stream().await?;
    let mut batch_stream = extractor.get_batch_proposed_stream().await?;

    info!("Processing events...");
    loop {
        tokio::select! {
            Some(block) = block_stream.next() => {
                info!("Processing block: {:?}", block.number);
                // Insert block into ClickHouse
                clickhouse_client.insert_block(&block).await?;
                info!("Inserted block: {:?}", block.number);
            }
            Some(batch) = batch_stream.next() => {
                info!("Processing batch: {:?}", batch.last_block_number());
                // TODO: Insert batch into ClickHouse
            }
        }
    }
}
