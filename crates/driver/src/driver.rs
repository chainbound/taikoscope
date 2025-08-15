//! Taikoscope Driver - combines ingestor and processor

use alloy_primitives::Address;
#[allow(unused_imports)]
use chainio::BatchesVerified;
use clickhouse::{ClickhouseReader, ClickhouseWriter};
use config::Opts;
use extractor::{
    BatchProposedStream, BatchesProvedStream, BatchesVerifiedStream, Extractor,
    ForcedInclusionStream, ReorgDetector,
};
use eyre::{Context, Result};
use incident::client::Client as IncidentClient;
use messages::TaikoEvent;
use primitives::headers::{L1HeaderStream, L2HeaderStream};
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tracing::{error, info, warn};
use url::Url;

use crate::subscription::subscribe_with_retry;

/// Driver that combines ingestor and processor functionality
#[derive(Debug)]
#[allow(dead_code)]
#[allow(missing_docs)]
pub struct Driver {
    pub extractor: Extractor,
    pub clickhouse_writer: Option<ClickhouseWriter>,
    pub clickhouse_reader: Option<ClickhouseReader>,
    pub reorg_detector: ReorgDetector,
    pub last_l2_header: Option<(u64, Address)>,
    pub enable_db_writes: bool,
    pub enable_gap_detection: bool,
    pub gap_finalization_buffer_blocks: u64,
    pub gap_startup_lookback_blocks: u64,
    pub gap_continuous_lookback_blocks: u64,
    pub gap_poll_interval_secs: u64,
    pub incident_client: IncidentClient,
    pub instatus_batch_submission_component_id: String,
    pub instatus_proof_submission_component_id: String,
    pub instatus_proof_verification_component_id: String,
    pub instatus_transaction_sequencing_component_id: String,
    pub instatus_public_api_component_id: String,
    pub instatus_monitors_enabled: bool,
    pub instatus_monitor_poll_interval_secs: u64,
    pub instatus_l1_monitor_threshold_secs: u64,
    pub instatus_l2_monitor_threshold_secs: u64,
    pub batch_proof_timeout_secs: u64,
    pub public_rpc_url: Option<Url>,
    pub inbox_address: Address,
    pub taiko_wrapper_address: Address,
}

impl Driver {
    /// Create a new driver with the given configuration
    pub async fn new(opts: Opts) -> Result<Self> {
        info!("Initializing driver");

        // verify monitoring configuration before doing any heavy work
        if opts.instatus.monitors_enabled && !opts.instatus.enabled() {
            return Err(eyre::eyre!(
                "Instatus configuration missing; set the INSTATUS_* environment variables"
            ));
        }

        // Validate ClickHouse configuration when database writes are enabled
        if opts.enable_db_writes {
            if opts.clickhouse.url.as_str().is_empty() {
                return Err(eyre::eyre!(
                    "ClickHouse URL is required when database writes are enabled"
                ));
            }
            if opts.clickhouse.db.is_empty() {
                return Err(eyre::eyre!(
                    "ClickHouse database name is required when database writes are enabled"
                ));
            }
            if opts.clickhouse.username.is_empty() {
                return Err(eyre::eyre!(
                    "ClickHouse username is required when database writes are enabled"
                ));
            }
            // Note: password can be empty for some configurations, so we don't validate it
        }

        if !opts.instatus.monitors_enabled {
            info!("Instatus monitors disabled; no incidents will be reported");
        }

        let extractor = Extractor::new(
            opts.rpc.l1_url.clone(),
            opts.rpc.l2_url.clone(),
            opts.taiko_addresses.inbox_address,
            opts.taiko_addresses.preconf_whitelist_address,
            opts.taiko_addresses.taiko_wrapper_address,
        )
        .await
        .wrap_err("Failed to initialize blockchain extractor. Ensure RPC URLs are WebSocket endpoints (ws:// or wss://)")?;

        // Always create a ClickhouseWriter for migrations, regardless of enable_db_writes
        let migration_writer = ClickhouseWriter::new(
            opts.clickhouse.url.clone(),
            opts.clickhouse.db.clone(),
            opts.clickhouse.username.clone(),
            opts.clickhouse.password.clone(),
        );

        // Handle dry-run mode (when database writes are disabled)
        if !opts.enable_db_writes {
            info!("🧪 DRY-RUN MODE: Database writes disabled");
            info!("   - Events will be processed and logged but not written to database");
            info!("   - Gap detection will run but not perform backfill operations");
            info!("   - All database writes will be simulated with detailed logging");
            info!("⚠️  Skipping database migrations (database writes disabled)");
        } else if opts.skip_migrations {
            info!("⚠️  Skipping database migrations");
        } else {
            info!("🚀 Running database migrations...");
            migration_writer.init_db(opts.reset_db).await?;
            info!("✅ Database migrations completed");
        }

        // Only keep the writer for event processing if database writes are enabled
        let clickhouse_writer = opts.enable_db_writes.then(|| {
            ClickhouseWriter::new(
                opts.clickhouse.url.clone(),
                opts.clickhouse.db.clone(),
                opts.clickhouse.username.clone(),
                opts.clickhouse.password.clone(),
            )
        });

        // Create ClickhouseReader for reorg detection (only if database writes are enabled)
        let clickhouse_reader = opts
            .enable_db_writes
            .then(|| {
                ClickhouseReader::new(
                    opts.clickhouse.url,
                    opts.clickhouse.db,
                    opts.clickhouse.username,
                    opts.clickhouse.password,
                )
            })
            .transpose()?;

        // Initialize reorg detector
        let reorg_detector = ReorgDetector::new();

        // init incident client and component IDs if monitors are enabled
        let (
            instatus_batch_submission_component_id,
            instatus_proof_submission_component_id,
            instatus_proof_verification_component_id,
            instatus_transaction_sequencing_component_id,
            instatus_public_api_component_id,
            incident_client,
        ) = if opts.instatus.monitors_enabled {
            (
                opts.instatus.batch_submission_component_id.clone(),
                opts.instatus.proof_submission_component_id.clone(),
                opts.instatus.proof_verification_component_id.clone(),
                opts.instatus.transaction_sequencing_component_id.clone(),
                opts.instatus.public_api_component_id.clone(),
                IncidentClient::new(opts.instatus.api_key.clone(), opts.instatus.page_id.clone()),
            )
        } else {
            (
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                IncidentClient::new(String::new(), String::new()),
            )
        };

        Ok(Self {
            extractor,
            clickhouse_writer,
            clickhouse_reader,
            reorg_detector,
            last_l2_header: None,
            enable_db_writes: opts.enable_db_writes,
            enable_gap_detection: opts.enable_gap_detection,
            gap_finalization_buffer_blocks: opts.gap_finalization_buffer_blocks,
            gap_startup_lookback_blocks: opts.gap_startup_lookback_blocks,
            gap_continuous_lookback_blocks: opts.gap_continuous_lookback_blocks,
            gap_poll_interval_secs: opts.gap_poll_interval_secs,
            incident_client,
            instatus_batch_submission_component_id,
            instatus_proof_submission_component_id,
            instatus_proof_verification_component_id,
            instatus_transaction_sequencing_component_id,
            instatus_public_api_component_id,
            instatus_monitors_enabled: opts.instatus.monitors_enabled,
            instatus_monitor_poll_interval_secs: opts.instatus.monitor_poll_interval_secs,
            instatus_l1_monitor_threshold_secs: opts.instatus.l1_monitor_threshold_secs,
            instatus_l2_monitor_threshold_secs: opts.instatus.l2_monitor_threshold_secs,
            batch_proof_timeout_secs: opts.instatus.batch_proof_timeout_secs,
            public_rpc_url: opts.rpc.public_url,
            inbox_address: opts.taiko_addresses.inbox_address,
            taiko_wrapper_address: opts.taiko_addresses.taiko_wrapper_address,
        })
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

    /// Start the driver event loop
    pub async fn start(self) -> Result<()> {
        self.start_with_shutdown(None).await
    }

    /// Start the driver event loop with graceful shutdown support
    pub async fn start_with_shutdown(
        mut self,
        shutdown_rx: Option<broadcast::Receiver<()>>,
    ) -> Result<()> {
        info!("Starting driver event loop");

        // Perform initial gap catch-up before starting live streams
        if self.enable_gap_detection {
            info!("Performing initial gap catch-up...");
            if let Err(e) = self.initial_gap_catchup().await {
                error!(err = %e, "Initial gap catch-up failed");
            } else {
                info!("Initial gap catch-up completed");
            }
        }

        // Start monitors if enabled
        let monitor_handles =
            if self.instatus_monitors_enabled { self.start_monitors().await } else { Vec::new() };

        // Start gap detection task if enabled
        let gap_detection_handle = if self.enable_gap_detection {
            self.start_gap_detection_task().await
        } else {
            info!("Gap detection disabled via configuration");
            None
        };

        let l1_stream = self.get_l1_headers().await;
        let l2_stream = self.get_l2_headers().await;
        let batch_stream = self.get_batch_proposed().await;
        let forced_stream = self.get_forced_inclusion().await;
        let proved_stream = self.get_batches_proved().await;
        let verified_stream = self.get_batches_verified().await;

        let result = self
            .event_loop(
                l1_stream,
                l2_stream,
                batch_stream,
                forced_stream,
                proved_stream,
                verified_stream,
                shutdown_rx,
            )
            .await;

        // Clean up monitors and gap detection
        for handle in monitor_handles {
            handle.abort();
        }
        if let Some(handle) = gap_detection_handle {
            handle.abort();
        }

        result
    }

    #[allow(clippy::too_many_arguments)]
    async fn event_loop(
        &mut self,
        mut l1_stream: L1HeaderStream,
        mut l2_stream: L2HeaderStream,
        mut batch_stream: BatchProposedStream,
        mut forced_stream: ForcedInclusionStream,
        mut proved_stream: BatchesProvedStream,
        mut verified_stream: BatchesVerifiedStream,
        mut shutdown_rx: Option<broadcast::Receiver<()>>,
    ) -> Result<()> {
        info!("Starting event loop - processing events directly to database");

        loop {
            tokio::select! {
                // Check for shutdown signal
                _ = async {
                    if let Some(ref mut shutdown_rx) = shutdown_rx {
                        shutdown_rx.recv().await
                    } else {
                        std::future::pending().await
                    }
                } => {
                    info!("Received shutdown signal, stopping event loop");
                    break;
                }

                maybe_l1 = l1_stream.next() => {
                    match maybe_l1 {
                        Some(header) => {
                            info!(block_number = header.number, hash = %header.hash, "Processing L1 header");
                            let event = TaikoEvent::L1Header(header);
                            if let Err(e) = self.process_event(event).await {
                                error!(err = %e, "Failed to process L1Header");
                            }
                        }
                        None => {
                            warn!("L1 header stream ended; re-subscribing…");
                            l1_stream = self.get_l1_headers().await;
                        }
                    }
                }
                maybe_l2 = l2_stream.next() => {
                    match maybe_l2 {
                        Some(header) => {
                            info!(block_number = header.number, hash = %header.hash, "Processing L2 header");
                            let event = TaikoEvent::L2Header(header);
                            if let Err(e) = self.process_event(event).await {
                                error!(err = %e, "Failed to process L2Header");
                            }
                        }
                        None => {
                            warn!("L2 header stream ended; re-subscribing…");
                            l2_stream = self.get_l2_headers().await;
                        }
                    }
                }
                maybe_batch = batch_stream.next() => {
                    match maybe_batch {
                        Some((batch, l1_tx_hash)) => {
                            info!(block_number = batch.last_block_number(), "Processing BatchProposed");
                            let wrapper = messages::BatchProposedWrapper::from((batch, l1_tx_hash, false));
                            let event = TaikoEvent::BatchProposed(wrapper);
                            if let Err(e) = self.process_event(event).await {
                                error!(err = %e, "Failed to process BatchProposed");
                            }
                        }
                        None => {
                            warn!("Batch proposed stream ended; re-subscribing…");
                            batch_stream = self.get_batch_proposed().await;
                        }
                    }
                }
                maybe_fi = forced_stream.next() => {
                    match maybe_fi {
                        Some(fi) => {
                            info!(blob_hash = ?fi.forcedInclusion.blobHash, "Processing forced inclusion processed");
                            let wrapper = messages::ForcedInclusionProcessedWrapper::from((fi, false));
                            let event = TaikoEvent::ForcedInclusionProcessed(wrapper);
                            if let Err(e) = self.process_event(event).await {
                                error!(err = %e, "Failed to process ForcedInclusionProcessed");
                            }
                        }
                        None => {
                            warn!("Forced inclusion stream ended; re-subscribing…");
                            forced_stream = self.get_forced_inclusion().await;
                        }
                    }
                }
                maybe_proved = proved_stream.next() => {
                    match maybe_proved {
                        Some((proved, l1_block_number, l1_tx_hash)) => {
                            info!(batch_ids = ?proved.batch_ids_proved(), "Processing batches proved");
                            let wrapper = messages::BatchesProvedWrapper::from((proved, l1_block_number, l1_tx_hash, false));
                            let event = TaikoEvent::BatchesProved(wrapper);
                            if let Err(e) = self.process_event(event).await {
                                error!(err = %e, "Failed to process BatchesProved");
                            }
                        }
                        None => {
                            warn!("Batches proved stream ended; re-subscribing…");
                            proved_stream = self.get_batches_proved().await;
                        }
                    }
                }
                maybe_verified = verified_stream.next() => {
                    match maybe_verified {
                        Some((verified, l1_block_number, l1_tx_hash)) => {
                            info!(batch_ids = ?verified.batch_id(), "Processing batches verified");
                            let wrapper = messages::BatchesVerifiedWrapper::from((verified, l1_block_number, l1_tx_hash, false));
                            let event = TaikoEvent::BatchesVerified(wrapper);
                            if let Err(e) = self.process_event(event).await {
                                error!(err = %e, "Failed to process BatchesVerified");
                            }
                        }
                        None => {
                            warn!("Batches verified stream ended; re-subscribing…");
                            verified_stream = self.get_batches_verified().await;
                        }
                    }
                }
                else => {
                    error!("All event streams ended and failed to re-subscribe. Shutting down driver loop");
                    break;
                }
            }
        }
        Ok(())
    }
}
