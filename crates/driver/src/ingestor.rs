//! Taikoscope Ingestor Driver

use eyre::Result;
use tokio_stream::StreamExt;
use tracing::info;

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
        .await?;

        let nats_client = async_nats::connect(&opts.nats_url).await?;
        info!("Connected to NATS server at {}", opts.nats_url);

        let jetstream = async_nats::jetstream::new(nats_client);
        jetstream
            .get_or_create_stream(async_nats::jetstream::stream::Config {
                name: "taiko".to_owned(),
                subjects: vec!["taiko.events".to_owned()],
                ..Default::default()
            })
            .await?;

        Ok(Self { extractor, jetstream })
    }

    async fn get_l1_headers(&self) -> Result<L1HeaderStream> {
        self.extractor.get_l1_header_stream().await
    }

    async fn get_l2_headers(&self) -> Result<L2HeaderStream> {
        self.extractor.get_l2_header_stream().await
    }

    async fn get_batch_proposed(&self) -> Result<BatchProposedStream> {
        self.extractor.get_batch_proposed_stream().await
    }

    async fn get_forced_inclusion(&self) -> Result<ForcedInclusionStream> {
        self.extractor.get_forced_inclusion_stream().await
    }

    async fn get_batches_proved(&self) -> Result<BatchesProvedStream> {
        self.extractor.get_batches_proved_stream().await
    }

    async fn get_batches_verified(&self) -> Result<BatchesVerifiedStream> {
        self.extractor.get_batches_verified_stream().await
    }

    /// Start the ingestor event loop, extracting events and publishing to NATS
    pub async fn start(self) -> Result<()> {
        info!("Starting ingestor event loop");

        let l1_stream = self.get_l1_headers().await?;
        let l2_stream = self.get_l2_headers().await?;
        let batch_stream = self.get_batch_proposed().await?;
        let forced_stream = self.get_forced_inclusion().await?;
        let proved_stream = self.get_batches_proved().await?;
        let verified_stream = self.get_batches_verified().await?;

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
        loop {
            tokio::select! {
                maybe_l1 = l1_stream.next() => {
                    if let Some(header) = maybe_l1 {
                        let event = TaikoEvent::L1Header(header);
                        if let Err(e) = publish_event_with_retry(&self.jetstream, &event, 3).await {
                            tracing::error!(err = %e, "Failed to publish L1Header");
                        }
                    }
                }
                maybe_l2 = l2_stream.next() => {
                    if let Some(header) = maybe_l2 {
                        let event = TaikoEvent::L2Header(header);
                        if let Err(e) = publish_event_with_retry(&self.jetstream, &event, 3).await {
                            tracing::error!(err = %e, "Failed to publish L2Header");
                        }
                    }
                }
                maybe_batch = batch_stream.next() => {
                    if let Some((batch, l1_tx_hash)) = maybe_batch {
                        let wrapper = BatchProposedWrapper::from((batch, l1_tx_hash));
                        let event = TaikoEvent::BatchProposed(wrapper);
                        if let Err(e) = publish_event_with_retry(&self.jetstream, &event, 3).await {
                            tracing::error!(err = %e, "Failed to publish BatchProposed");
                        }
                    }
                }
                maybe_fi = forced_stream.next() => {
                    if let Some(fi) = maybe_fi {
                        let wrapper = ForcedInclusionProcessedWrapper::from(fi);
                        let event = TaikoEvent::ForcedInclusionProcessed(wrapper);
                        if let Err(e) = publish_event_with_retry(&self.jetstream, &event, 3).await {
                            tracing::error!(err = %e, "Failed to publish ForcedInclusionProcessed");
                        }
                    }
                }
                maybe_proved = proved_stream.next() => {
                    if let Some((proved, l1_block_number, l1_tx_hash)) = maybe_proved {
                        let wrapper = BatchesProvedWrapper::from((proved, l1_block_number, l1_tx_hash));
                        let event = TaikoEvent::BatchesProved(wrapper);
                        if let Err(e) = publish_event_with_retry(&self.jetstream, &event, 3).await {
                            tracing::error!(err = %e, "Failed to publish BatchesProved");
                        }
                    }
                }
                maybe_verified = verified_stream.next() => {
                    if let Some((verified, l1_block_number, l1_tx_hash)) = maybe_verified {
                        let wrapper = BatchesVerifiedWrapper::from((verified, l1_block_number, l1_tx_hash));
                        let event = TaikoEvent::BatchesVerified(wrapper);
                        if let Err(e) = publish_event_with_retry(&self.jetstream, &event, 3).await {
                            tracing::error!(err = %e, "Failed to publish BatchesVerified");
                        }
                    }
                }
            }
        }
    }
}
