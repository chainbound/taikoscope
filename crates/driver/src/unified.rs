//! Taikoscope Unified Driver - combines ingestor and processor without NATS

use std::collections::VecDeque;

use alloy_primitives::{Address, BlockHash};
use clickhouse::{ClickhouseReader, ClickhouseWriter};
use config::Opts;
use extractor::{
    BatchProposedStream, BatchesProvedStream, BatchesVerifiedStream, Extractor,
    ForcedInclusionStream, ReorgDetector,
};
use eyre::{Context, Result};
use incident::client::Client as IncidentClient;
use messages::{
    BatchProposedWrapper, BatchesProvedWrapper, BatchesVerifiedWrapper,
    ForcedInclusionProcessedWrapper,
};
use nats_utils::TaikoEvent;
use primitives::headers::{L1HeaderStream, L2HeaderStream};
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tracing::{error, info, warn};
use url::Url;

use crate::subscription::subscribe_with_retry;

/// Unified driver that combines ingestor and processor functionality without NATS
#[derive(Debug)]
#[allow(dead_code)]
pub struct UnifiedDriver {
    extractor: Extractor,
    clickhouse_writer: Option<ClickhouseWriter>,
    clickhouse_reader: Option<ClickhouseReader>,
    reorg_detector: ReorgDetector,
    last_l2_header: Option<(u64, Address)>,
    enable_db_writes: bool,
    incident_client: IncidentClient,
    instatus_batch_submission_component_id: String,
    instatus_proof_submission_component_id: String,
    instatus_proof_verification_component_id: String,
    instatus_transaction_sequencing_component_id: String,
    instatus_public_api_component_id: String,
    instatus_monitors_enabled: bool,
    instatus_monitor_poll_interval_secs: u64,
    instatus_l1_monitor_threshold_secs: u64,
    instatus_l2_monitor_threshold_secs: u64,
    batch_proof_timeout_secs: u64,
    public_rpc_url: Option<Url>,
    processed_l2_headers: VecDeque<BlockHash>,
}

/// An EPOCH is a series of 32 slots.
#[allow(dead_code)]
const EPOCH_SLOTS: u64 = 32;

impl UnifiedDriver {
    /// Create a new unified driver with the given configuration
    pub async fn new(opts: Opts) -> Result<Self> {
        info!("Initializing unified driver");

        // verify monitoring configuration before doing any heavy work
        if opts.instatus.monitors_enabled && !opts.instatus.enabled() {
            return Err(eyre::eyre!(
                "Instatus configuration missing; set the INSTATUS_* environment variables"
            ));
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

        if opts.skip_migrations {
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
            processed_l2_headers: VecDeque::new(),
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

    /// Start the unified driver event loop
    pub async fn start(self) -> Result<()> {
        self.start_with_shutdown(None).await
    }

    /// Start the unified driver event loop with graceful shutdown support
    pub async fn start_with_shutdown(
        mut self,
        shutdown_rx: Option<broadcast::Receiver<()>>,
    ) -> Result<()> {
        info!("Starting unified driver event loop");

        // Start monitors if enabled
        let monitor_handles =
            if self.instatus_monitors_enabled { self.start_monitors().await } else { Vec::new() };

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

        // Clean up monitors
        for handle in monitor_handles {
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
        info!("Starting unified event loop - processing events directly to database");

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
                            let wrapper = BatchProposedWrapper::from((batch, l1_tx_hash, false));
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
                            let wrapper = ForcedInclusionProcessedWrapper::from((fi, false));
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
                            let wrapper = BatchesProvedWrapper::from((proved, l1_block_number, l1_tx_hash, false));
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
                            let wrapper = BatchesVerifiedWrapper::from((verified, l1_block_number, l1_tx_hash, false));
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
                    error!("All event streams ended and failed to re-subscribe. Shutting down unified driver loop");
                    break;
                }
            }
        }
        Ok(())
    }

    /// Process an event directly to the database (replacing NATS processing)
    async fn process_event(&mut self, event: TaikoEvent) -> Result<()> {
        if !self.enable_db_writes {
            info!("Database writes disabled, would process event");
            return Ok(());
        }

        // Check writer exists early
        if self.clickhouse_writer.is_none() {
            warn!("No ClickHouse writer available");
            return Ok(());
        }

        // Process each event type with proper error handling
        match event {
            TaikoEvent::L1Header(header) => {
                info!(block_number = header.number, hash = %header.hash, "Processing L1 header");
                self.handle_l1_header_event(header).await
            }
            TaikoEvent::L2Header(header) => {
                info!(block_number = header.number, hash = %header.hash, "Processing L2 header");
                self.handle_l2_header_event(header).await
            }
            TaikoEvent::BatchProposed(wrapper) => {
                info!(last_block = wrapper.batch.last_block_number(), "Processing batch proposed");
                self.handle_batch_proposed_event(wrapper).await
            }
            TaikoEvent::ForcedInclusionProcessed(wrapper) => {
                info!(blob_hash = ?wrapper.event.forcedInclusion.blobHash, "Processing forced inclusion");
                self.handle_forced_inclusion_event(wrapper).await
            }
            TaikoEvent::BatchesProved(wrapper) => {
                info!(batch_ids = ?wrapper.proved.batch_ids_proved(), "Processing batches proved");
                self.handle_batches_proved_event(wrapper).await
            }
            TaikoEvent::BatchesVerified(wrapper) => {
                info!(batch_id = wrapper.verified.batch_id, "Processing batches verified");
                self.handle_batches_verified_event(wrapper).await
            }
        }
    }

    // Event handler methods
    async fn handle_l1_header_event(&self, header: primitives::headers::L1Header) -> Result<()> {
        let writer = self.clickhouse_writer.as_ref().unwrap();

        // Insert L1 header
        Self::with_db_error_context(
            writer.insert_l1_header(&header),
            "insert L1 header",
            format!("header_number={}", header.number),
        )
        .await?;

        // Process preconfirmation data
        self.process_preconf_data(&header).await;

        Ok(())
    }

    async fn handle_l2_header_event(
        &mut self,
        header: primitives::headers::L2Header,
    ) -> Result<()> {
        // Duplicate filtering using FIFO set
        if self.processed_l2_headers.contains(&header.hash) {
            warn!("Duplicate L2Header detected, skipping processing");
            return Ok(());
        }

        // Add to FIFO set (max 1000 items)
        self.processed_l2_headers.push_back(header.hash);
        if self.processed_l2_headers.len() > 1000 {
            self.processed_l2_headers.pop_front();
        }

        // Process reorg detection
        self.process_reorg_detection(&header).await;

        // Insert L2 header with block statistics
        self.insert_l2_header_with_stats(&header).await;

        Ok(())
    }

    async fn handle_batch_proposed_event(&self, wrapper: BatchProposedWrapper) -> Result<()> {
        let writer = self.clickhouse_writer.as_ref().unwrap();
        let batch = &wrapper.batch;
        let l1_tx_hash = wrapper.l1_tx_hash;

        // Insert batch with error handling
        Self::with_db_error_context(
            writer.insert_batch(batch, l1_tx_hash),
            "insert batch",
            format!("batch_last_block={:?}", batch.last_block_number()),
        )
        .await?;

        // Calculate and insert L1 data cost
        if let Some(cost) = Self::fetch_transaction_cost(&self.extractor, l1_tx_hash).await {
            Self::with_db_error_context(
                writer.insert_l1_data_cost(batch.info.proposedIn, batch.meta.batchId, cost),
                "insert L1 data cost",
                format!("l1_block_number={}, tx_hash={:?}", batch.info.proposedIn, l1_tx_hash),
            )
            .await?;
        }
        Ok(())
    }

    async fn handle_forced_inclusion_event(
        &self,
        wrapper: ForcedInclusionProcessedWrapper,
    ) -> Result<()> {
        let writer = self.clickhouse_writer.as_ref().unwrap();
        let event = &wrapper.event;

        Self::with_db_error_context(
            writer.insert_forced_inclusion(event),
            "insert forced inclusion",
            format!("blob_hash={:?}", event.forcedInclusion.blobHash),
        )
        .await?;

        Ok(())
    }

    async fn handle_batches_proved_event(&self, wrapper: BatchesProvedWrapper) -> Result<()> {
        let writer = self.clickhouse_writer.as_ref().unwrap();
        let proved = &wrapper.proved;
        let l1_block_number = wrapper.l1_block_number;
        let l1_tx_hash = wrapper.l1_tx_hash;

        // Insert proved batch
        Self::with_db_error_context(
            writer.insert_proved_batch(proved, l1_block_number),
            "insert proved batch",
            format!("batch_ids={:?}", proved.batch_ids_proved()),
        )
        .await?;

        // Calculate and insert prove costs for each batch
        if let Some(cost) = Self::fetch_transaction_cost(&self.extractor, l1_tx_hash).await {
            let cost_per_batch =
                Self::average_cost_per_batch(cost, proved.batch_ids_proved().len());

            for batch_id in proved.batch_ids_proved() {
                Self::with_db_error_context(
                    writer.insert_prove_cost(l1_block_number, *batch_id, cost_per_batch),
                    "insert prove cost",
                    format!(
                        "l1_block_number={}, batch_id={}, tx_hash={:?}",
                        l1_block_number, batch_id, l1_tx_hash
                    ),
                )
                .await?;
            }
        }
        Ok(())
    }

    async fn handle_batches_verified_event(&self, wrapper: BatchesVerifiedWrapper) -> Result<()> {
        let writer = self.clickhouse_writer.as_ref().unwrap();
        let verified = &wrapper.verified;
        let l1_block_number = wrapper.l1_block_number;
        let l1_tx_hash = wrapper.l1_tx_hash;

        // Insert verified batch
        Self::with_db_error_context(
            writer.insert_verified_batch(verified, l1_block_number),
            "insert verified batch",
            format!("batch_id={}", verified.batch_id),
        )
        .await?;

        // Calculate and insert verify cost
        if let Some(cost) = Self::fetch_transaction_cost(&self.extractor, l1_tx_hash).await {
            Self::with_db_error_context(
                writer.insert_verify_cost(l1_block_number, verified.batch_id, cost),
                "insert verify cost",
                format!(
                    "l1_block_number={}, batch_id={}, tx_hash={:?}",
                    l1_block_number, verified.batch_id, l1_tx_hash
                ),
            )
            .await?;
        }
        Ok(())
    }

    // Helper methods
    async fn with_db_error_context<F, T>(future: F, operation: &str, context: String) -> Result<T>
    where
        F: std::future::Future<Output = Result<T, eyre::Error>>,
    {
        future.await.map_err(|e| {
            error!(
                err = %e,
                operation = operation,
                context = context,
                "Database operation failed"
            );
            eyre::eyre!("Failed to {}: {} - {}", operation, context, e)
        })
    }

    async fn fetch_transaction_cost(
        extractor: &Extractor,
        tx_hash: alloy_primitives::B256,
    ) -> Option<u128> {
        if tx_hash == alloy_primitives::B256::ZERO {
            return None;
        }

        match extractor.get_receipt(tx_hash).await {
            Ok(receipt) => Some(primitives::l1_data_cost::cost_from_receipt(&receipt)),
            Err(e) => {
                warn!(err = %e, tx_hash = %tx_hash, "Failed to fetch transaction receipt");
                None
            }
        }
    }

    const fn average_cost_per_batch(total_cost: u128, num_batches: usize) -> u128 {
        if num_batches == 0 { 0 } else { total_cost / num_batches as u128 }
    }

    async fn process_preconf_data(&self, header: &primitives::headers::L1Header) {
        // TODO: Implement preconf data processing - placeholder for now
        info!(block_number = header.number, "Preconf data processing not yet implemented");
    }

    async fn process_reorg_detection(&self, header: &primitives::headers::L2Header) {
        // TODO: Implement reorg detection - placeholder for now
        info!(block_number = header.number, "Reorg detection not yet implemented");
    }

    async fn insert_l2_header_with_stats(&self, header: &primitives::headers::L2Header) {
        // TODO: Implement L2 header insertion with stats - placeholder for now
        info!(block_number = header.number, "L2 header with stats insertion not yet implemented");
    }

    /// Start monitoring tasks if enabled
    async fn start_monitors(&self) -> Vec<tokio::task::JoinHandle<()>> {
        let handles = Vec::new();

        // TODO: Implement monitors properly
        // For now, just return empty handles to avoid compilation errors
        info!("Monitors not yet implemented in unified mode");

        handles
    }
}
