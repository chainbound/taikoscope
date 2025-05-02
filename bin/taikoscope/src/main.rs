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
    let clickhouse_client = ClickhouseClient::new(opts.clickhouse_url)?;
    clickhouse_client.init_db().await?;

    info!("Initializing extractor...");
    let extractor = Extractor::new(
        opts.l1_rpc_url,
        opts.l2_rpc_url,
        opts.inbox_address,
        opts.preconf_whitelist_address,
        opts.taiko_wrapper_address,
    )
    .await?;

    let mut l1_header_stream = extractor.get_l1_header_stream().await?;
    let mut l2_header_stream = extractor.get_l2_header_stream().await?;
    let mut batch_stream = extractor.get_batch_proposed_stream().await?;
    let mut forced_inclusion_processed_stream =
        extractor.get_forced_inclusion_processed_stream().await?;

    info!("Processing events...");
    loop {
        tokio::select! {
            Some(header) = l1_header_stream.next() => {
                clickhouse_client.insert_l1_header(&header).await?;
                info!("Inserted L1 header: {:?}", header.number);

                let candidates = extractor.get_operator_candidates_for_current_epoch().await?;
                let current_operator = extractor.get_operator_for_current_epoch().await?;
                let next_operator = extractor.get_operator_for_next_epoch().await?;

                clickhouse_client.insert_preconf_data(header.slot, candidates, current_operator, next_operator).await?;
                info!("Inserted preconf data for slot: {:?}", header.slot);
            }
            Some(header) = l2_header_stream.next() => {
                clickhouse_client.insert_l2_header(&header).await?;
                info!("Inserted L2 header: {:?}", header.number);
            }
            Some(batch) = batch_stream.next() => {
                clickhouse_client.insert_batch(&batch).await?;
                info!("Inserted batch: {:?}", batch.last_block_number());
            }
            Some(forced_inclusion_processed) = forced_inclusion_processed_stream.next() => {
                // clickhouse_client.insert_forced_inclusion_processed(&forced_inclusion_processed).await?;
                info!("Inserted forced inclusion processed: {:?}", forced_inclusion_processed.forcedInclusion.blobHash);
            }
        }
    }
}
