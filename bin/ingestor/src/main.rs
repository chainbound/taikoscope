#![allow(missing_docs)]
#![allow(unused_imports)]

use clap::Parser;
use config::Opts;
use dotenvy::dotenv;
use extractor::{
    BatchProposedStream, BatchesProvedStream, BatchesVerifiedStream, Extractor,
    ForcedInclusionStream,
};
use nats_utils::{TaikoEvent, publish_event};
use primitives::headers::{L1HeaderStream, L2HeaderStream};
use tokio_stream::StreamExt;
use tracing::info;
use tracing_subscriber::filter::EnvFilter;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    if let Ok(custom_env_file) = std::env::var("ENV_FILE") {
        dotenvy::from_filename(custom_env_file)?;
    } else {
        dotenv().ok();
    }

    let opts = Opts::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("Ingestor starting...");

    // Connect to NATS
    let nats_client = async_nats::connect(&opts.nats_url).await?;

    // Set up Extractor
    let extractor = Extractor::new(
        opts.rpc.l1_url.clone(),
        opts.rpc.l2_url.clone(),
        opts.taiko_addresses.inbox_address,
        opts.taiko_addresses.preconf_whitelist_address,
        opts.taiko_addresses.taiko_wrapper_address,
    )
    .await?;

    // Subscribe to all event streams
    let mut l1_stream = extractor.get_l1_header_stream().await?;
    let mut l2_stream = extractor.get_l2_header_stream().await?;
    let mut batch_stream = extractor.get_batch_proposed_stream().await?;
    let mut forced_stream = extractor.get_forced_inclusion_stream().await?;
    let mut proved_stream = extractor.get_batches_proved_stream().await?;
    let mut verified_stream = extractor.get_batches_verified_stream().await?;

    loop {
        tokio::select! {
            maybe_l1 = l1_stream.next() => {
                if let Some(header) = maybe_l1 {
                    let event = TaikoEvent::L1Header(header);
                    if let Err(e) = publish_event(&nats_client, &event).await {
                        tracing::error!(err = %e, "Failed to publish L1Header");
                    }
                }
            }
            maybe_l2 = l2_stream.next() => {
                if let Some(header) = maybe_l2 {
                    let event = TaikoEvent::L2Header(header);
                    if let Err(e) = publish_event(&nats_client, &event).await {
                        tracing::error!(err = %e, "Failed to publish L2Header");
                    }
                }
            }
            maybe_batch = batch_stream.next() => {
                if let Some(batch_data) = maybe_batch {
                    let event = TaikoEvent::BatchProposed(batch_data.into());
                    if let Err(e) = publish_event(&nats_client, &event).await {
                        tracing::error!(err = %e, "Failed to publish BatchProposed");
                    }
                }
            }
            maybe_fi = forced_stream.next() => {
                if let Some(fi) = maybe_fi {
                    let event = TaikoEvent::ForcedInclusionProcessed(fi.into());
                    if let Err(e) = publish_event(&nats_client, &event).await {
                        tracing::error!(err = %e, "Failed to publish ForcedInclusionProcessed");
                    }
                }
            }
            maybe_proved = proved_stream.next() => {
                if let Some(proved) = maybe_proved {
                    let event = TaikoEvent::BatchesProved(proved.into());
                    if let Err(e) = publish_event(&nats_client, &event).await {
                        tracing::error!(err = %e, "Failed to publish BatchesProved");
                    }
                }
            }
            maybe_verified = verified_stream.next() => {
                if let Some(verified) = maybe_verified {
                    let event = TaikoEvent::BatchesVerified(verified.0);
                    if let Err(e) = publish_event(&nats_client, &event).await {
                        tracing::error!(err = %e, "Failed to publish BatchesVerified");
                    }
                }
            }
        }
    }
}
