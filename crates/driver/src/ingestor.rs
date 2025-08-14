//! Taikoscope Ingestor Driver

use eyre::{Context, Result};
use tokio_stream::StreamExt;
use tracing::info;

use crate::subscription::subscribe_with_retry;
use config::Opts;
use extractor::{
    BatchProposedStream, BatchesProvedStream, BatchesVerifiedStream, Extractor,
    ForcedInclusionStream,
};
use messages::{
    BatchProposedWrapper, BatchesProvedWrapper, BatchesVerifiedWrapper,
    ForcedInclusionProcessedWrapper,
};
use nats_utils::{TaikoEvent, publish_event_with_retry};
use primitives::headers::{L1HeaderStream, L2HeaderStream};

/// Driver for the ingestor service that extracts blockchain events and publishes them to NATS
#[derive(Debug)]
pub struct IngestorDriver {
    extractor: Extractor,
    jetstream: async_nats::jetstream::Context,
}

impl IngestorDriver {
    /// Create a new ingestor driver with the given configuration
    pub async fn new(opts: Opts) -> Result<Self> {
        info!("Initializing ingestor driver");

        let extractor = Extractor::new(
            opts.rpc.l1_url,
            opts.rpc.l2_url,
            opts.taiko_addresses.inbox_address,
            opts.taiko_addresses.preconf_whitelist_address,
            opts.taiko_addresses.taiko_wrapper_address,
        )
        .await
        .wrap_err("Failed to initialize blockchain extractor. Ensure RPC URLs are WebSocket endpoints (ws:// or wss://)")?;

        let nats_client = async_nats::connect(&opts.nats_url)
            .await
            .wrap_err_with(|| format!("failed to connect to NATS at {}", opts.nats_url))?;
        info!("Connected to NATS server at {}", opts.nats_url);

        let jetstream = async_nats::jetstream::new(nats_client);
        jetstream
            .get_or_create_stream(async_nats::jetstream::stream::Config {
                name: "taiko".to_owned(),
                subjects: vec!["taiko.events".to_owned()],
                duplicate_window: opts.nats_stream.get_duplicate_window(),
                storage: opts.nats_stream.get_storage_type(),
                retention: opts.nats_stream.get_retention_policy(),
                ..Default::default()
            })
            .await?;

        Ok(Self { extractor, jetstream })
    }

    async fn get_l1_headers(&self) -> L1HeaderStream {
        subscribe_with_retry(|| self.extractor.get_l1_header_stream(), "l1 headers").await
    }

    async fn get_l2_headers(&self) -> L2HeaderStream {
        subscribe_with_retry(|| self.extractor.get_l2_header_stream(), "l2 headers").await
    }

    async fn get_batch_proposed(&self) -> BatchProposedStream {
        subscribe_with_retry(|| self.extractor.get_batch_proposed_stream(), "batch proposed").await
    }

    async fn get_forced_inclusion(&self) -> ForcedInclusionStream {
        subscribe_with_retry(|| self.extractor.get_forced_inclusion_stream(), "forced inclusion")
            .await
    }

    async fn get_batches_proved(&self) -> BatchesProvedStream {
        subscribe_with_retry(|| self.extractor.get_batches_proved_stream(), "batches proved").await
    }

    async fn get_batches_verified(&self) -> BatchesVerifiedStream {
        subscribe_with_retry(|| self.extractor.get_batches_verified_stream(), "batches verified")
            .await
    }

    /// Start the ingestor event loop, extracting events and publishing to NATS
    pub async fn start(self) -> Result<()> {
        info!("Starting ingestor event loop");

        let l1_stream = self.get_l1_headers().await;
        let l2_stream = self.get_l2_headers().await;
        let batch_stream = self.get_batch_proposed().await;
        let forced_stream = self.get_forced_inclusion().await;
        let proved_stream = self.get_batches_proved().await;
        let verified_stream = self.get_batches_verified().await;

        self.event_loop(
            l1_stream,
            l2_stream,
            batch_stream,
            forced_stream,
            proved_stream,
            verified_stream,
        )
        .await
    }

    async fn event_loop(
        self,
        mut l1_stream: L1HeaderStream,
        mut l2_stream: L2HeaderStream,
        mut batch_stream: BatchProposedStream,
        mut forced_stream: ForcedInclusionStream,
        mut proved_stream: BatchesProvedStream,
        mut verified_stream: BatchesVerifiedStream,
    ) -> Result<()> {
        info!("Starting ingestor event loop");

        loop {
            tokio::select! {
                maybe_l1 = l1_stream.next() => {
                    match maybe_l1 {
                        Some(header) => {
                            info!(block_number = header.number, hash = %header.hash, "Publishing L1 header");
                            let event = TaikoEvent::L1Header(header);
                            if let Err(e) = publish_event_with_retry(&self.jetstream, &event, 10).await {
                                tracing::error!(err = %e, "Failed to publish L1Header");
                            }
                        }
                        None => {
                            tracing::warn!("L1 header stream ended; re-subscribing…");
                            l1_stream = self.get_l1_headers().await;
                        }
                    }
                }
                maybe_l2 = l2_stream.next() => {
                    match maybe_l2 {
                        Some(header) => {
                            info!(block_number = header.number, hash = %header.hash, "Publishing L2 header");
                            let event = TaikoEvent::L2Header(header);
                            if let Err(e) = publish_event_with_retry(&self.jetstream, &event, 10).await {
                                tracing::error!(err = %e, "Failed to publish L2Header");
                            }
                        }
                        None => {
                            tracing::warn!("L2 header stream ended; re-subscribing…");
                            l2_stream = self.get_l2_headers().await;
                        }
                    }
                }
                maybe_batch = batch_stream.next() => {
                    match maybe_batch {
                        Some((batch, l1_tx_hash)) => {
                            info!(block_number = batch.last_block_number(), "Publishing BatchProposed");
                            let wrapper = BatchProposedWrapper::from((batch, l1_tx_hash, false));
                            let event = TaikoEvent::BatchProposed(wrapper);
                            if let Err(e) = publish_event_with_retry(&self.jetstream, &event, 10).await {
                                tracing::error!(err = %e, "Failed to publish BatchProposed");
                            }
                        }
                        None => {
                            tracing::warn!("Batch proposed stream ended; re-subscribing…");
                            batch_stream = self.get_batch_proposed().await;
                        }
                    }
                }
                maybe_fi = forced_stream.next() => {
                    match maybe_fi {
                        Some(fi) => {
                            info!(blob_hash = ?fi.forcedInclusion.blobHash, "Publishing forced inclusion processed");
                            let wrapper = ForcedInclusionProcessedWrapper::from((fi, false));
                            let event = TaikoEvent::ForcedInclusionProcessed(wrapper);
                            if let Err(e) = publish_event_with_retry(&self.jetstream, &event, 10).await {
                                tracing::error!(err = %e, "Failed to publish ForcedInclusionProcessed");
                            }
                        }
                        None => {
                            tracing::warn!("Forced inclusion stream ended; re-subscribing…");
                            forced_stream = self.get_forced_inclusion().await;
                        }
                    }
                }
                maybe_proved = proved_stream.next() => {
                    match maybe_proved {
                        Some((proved, l1_block_number, l1_tx_hash)) => {
                            info!(batch_ids = ?proved.batch_ids_proved(), "Publishing batches proved");
                            let wrapper = BatchesProvedWrapper::from((proved, l1_block_number, l1_tx_hash, false));
                            let event = TaikoEvent::BatchesProved(wrapper);
                            if let Err(e) = publish_event_with_retry(&self.jetstream, &event, 10).await {
                                tracing::error!(err = %e, "Failed to publish BatchesProved");
                            }
                        }
                        None => {
                            tracing::warn!("Batches proved stream ended; re-subscribing…");
                            proved_stream = self.get_batches_proved().await;
                        }
                    }
                }
                maybe_verified = verified_stream.next() => {
                    match maybe_verified {
                        Some((verified, l1_block_number, l1_tx_hash)) => {
                            info!(batch_ids = ?verified.batch_id(), "Publishing batches verified");
                            let wrapper = BatchesVerifiedWrapper::from((verified, l1_block_number, l1_tx_hash, false));
                            let event = TaikoEvent::BatchesVerified(wrapper);
                            if let Err(e) = publish_event_with_retry(&self.jetstream, &event, 10).await {
                                tracing::error!(err = %e, "Failed to publish BatchesVerified");
                            }
                        }
                        None => {
                            tracing::warn!("Batches verified stream ended; re-subscribing…");
                            verified_stream = self.get_batches_verified().await;
                        }
                    }
                }
                else => {
                    tracing::error!("All event streams ended and failed to re-subscribe. Shutting down ingestor loop");
                    break;
                }
            }
        }
        Ok(())
    }
}
