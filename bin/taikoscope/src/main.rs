//! Entrypoint.

use extractor::{Extractor, ReorgDetector};
use inserter::ClickhouseClient;
use tokio_stream::StreamExt;

use clap::Parser;
use config::Opts;
use dotenvy::dotenv;
use tracing::info;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    if let Ok(custom_env_file) = std::env::var("ENV_FILE") {
        dotenvy::from_filename(custom_env_file)?;
    } else {
        // Try the default .env file, and ignore if it doesn't exist.
        dotenv().ok();
    }

    let opts = Opts::parse();

    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    info!("Initializing ClickHouse client...");
    let clickhouse_client = ClickhouseClient::new(
        opts.clickhouse_url,
        opts.clickhouse_db,
        opts.clickhouse_username,
        opts.clickhouse_password,
    )?;
    clickhouse_client.init_db(opts.reset_db).await?;

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
    let mut forced_inclusion_stream = extractor.get_forced_inclusion_stream().await?;

    let mut reorg_detector = ReorgDetector::new();

    info!("Processing events...");
    loop {
        tokio::select! {
            Some(header) = l1_header_stream.next() => {
                clickhouse_client.insert_l1_header(&header).await?;
                info!("Inserted L1 header: {:?}", header.number);

                // TODO: uncomment this when this is deployed
                /*
                let opt_candidates = match extractor.get_operator_candidates_for_current_epoch().await {
                    Ok(c) => Some(c),
                    Err(e) => {
                        tracing::error!(
                            slot = header.slot,
                            block = header.number,
                            err = %e,
                            "Failed picking operator candidates"
                        );
                        None
                    }
                };
                */
                let opt_candidates = Some(vec![]);

                let opt_current_operator = match extractor.get_operator_for_current_epoch().await {
                    Ok(op) => Some(op),
                    Err(e) => {
                        tracing::error!(block = header.number, err = %e, "get_operator_for_current_epoch failed");
                        None
                    }
                };

                let opt_next_operator = match extractor.get_operator_for_next_epoch().await {
                    Ok(op) => Some(op),
                    Err(e) => {
                        tracing::error!(block = header.number, err = %e, "get_operator_for_next_epoch failed");
                        None
                    }
                };

                if let (Some(candidates), Some(current_operator), Some(next_operator)) =
                    (opt_candidates, opt_current_operator, opt_next_operator)
                {
                    clickhouse_client.insert_preconf_data(header.slot, candidates, current_operator, next_operator).await?;
                    info!("Inserted preconf data for slot: {:?}", header.slot);
                } else {
                    info!("Skipping preconf data insertion for slot {:?} due to errors fetching operator data.", header.slot);
                }
            }
            Some(header) = l2_header_stream.next() => {
                // Detect reorgs
                if let Some((hash, old_hash, depth)) = reorg_detector.on_new_block(header.number, header.hash, header.parent_hash) {
                    clickhouse_client.insert_l2_reorg(header.number, hash, old_hash, depth).await?;
                    info!("Inserted L2 reorg: {:?}", header.number);
                } else {
                    clickhouse_client.insert_l2_header(&header).await?;
                    info!("Inserted L2 header: {:?}", header.number);
                }
            }
            Some(batch) = batch_stream.next() => {
                clickhouse_client.insert_batch(&batch).await?;
                info!("Inserted batch: {:?}", batch.last_block_number());
            }
            Some(forced_inclusion_processed) = forced_inclusion_stream.next() => {
                clickhouse_client.insert_forced_inclusion(&forced_inclusion_processed).await?;
                info!("Inserted forced inclusion processed: {:?}", forced_inclusion_processed.forcedInclusion.blobHash);
            }
        }
    }
}
