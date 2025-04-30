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

    let mut l1_header_stream = extractor.get_l1_header_stream().await?;
    let mut l2_header_stream = extractor.get_l2_header_stream().await?;
    let mut batch_stream = extractor.get_batch_proposed_stream().await?;

    info!("Processing events...");
    loop {
        tokio::select! {
            Some(header) = l1_header_stream.next() => {
                info!("Processing L1 header: {:?}", header.number);
                clickhouse_client.insert_l1_header(&header).await?;
                info!("Inserted L1 header: {:?}", header.number);
            }
            Some(header) = l2_header_stream.next() => {
                info!("Processing L2 header: {:?}", header.number);
                clickhouse_client.insert_l2_header(&header).await?;
                info!("Inserted L2 header: {:?}", header.number);
            }
            Some(batch) = batch_stream.next() => {
                info!("Processing batch: {:?}", batch.last_block_number());
                // TODO: Insert batch into ClickHouse
            }
        }
    }
}
