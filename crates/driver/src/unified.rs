//! Taikoscope Unified Driver - combines ingestor and processor without NATS

use std::{collections::VecDeque, time::Duration};

use alloy_primitives::{Address, BlockHash};
#[allow(unused_imports)]
use chainio::BatchesVerified;
use clickhouse::{AddressBytes, ClickhouseReader, ClickhouseWriter, HashBytes, L2HeadEvent};
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
    // Contract addresses for event filtering
    inbox_address: Address,
    taiko_wrapper_address: Address,
}

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

        // Handle dry-run mode (when database writes are disabled)
        if !opts.enable_db_writes {
            info!("🧪 DRY-RUN MODE: Database writes disabled");
            info!("   - Events will be processed and logged but not written to database");
            info!("   - Gap detection will run but not perform backfill operations");
            info!("   - All database writes will be simulated with detailed logging");
        }

        // Skip migrations when database writes are disabled
        if !opts.enable_db_writes {
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

        // Start gap detection task
        let gap_detection_handle = self.start_gap_detection_task().await;

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
        // Handle dry-run mode with detailed logging
        if !self.enable_db_writes {
            return self.process_event_dry_run(event).await;
        }

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

    /// Process an event in dry-run mode with detailed logging but no database writes
    async fn process_event_dry_run(&mut self, event: TaikoEvent) -> Result<()> {
        match event {
            TaikoEvent::L1Header(header) => {
                info!(
                    block_number = header.number,
                    hash = %header.hash,
                    slot = header.slot,
                    timestamp = header.timestamp,
                    "🧪 DRY-RUN: Would process L1 header"
                );

                // Simulate preconf data processing
                info!(
                    block_number = header.number,
                    "🧪 DRY-RUN: Would insert L1 header and process preconf data"
                );

                // Still run preconf data logic for validation (but won't write to DB)
                self.process_preconf_data(&header).await;

                Ok(())
            }
            TaikoEvent::L2Header(header) => {
                info!(
                    block_number = header.number,
                    hash = %header.hash,
                    beneficiary = %header.beneficiary,
                    gas_used = header.gas_used,
                    base_fee = header.base_fee_per_gas,
                    timestamp = header.timestamp,
                    "🧪 DRY-RUN: Would process L2 header"
                );

                // Still run reorg detection for validation (but won't write to DB)
                self.process_reorg_detection(&header).await;

                // Simulate stats calculation
                let (sum_gas_used, sum_tx, sum_priority_fee) = self.extractor
                    .get_l2_block_stats(alloy_primitives::B256::from(*header.hash), header.base_fee_per_gas)
                    .await
                    .unwrap_or_else(|e| {
                        warn!(header_number = header.number, err = %e, "🧪 DRY-RUN: Failed to get L2 block stats");
                        (0, 0, 0)
                    });

                let sum_base_fee = sum_gas_used.saturating_mul(header.base_fee_per_gas as u128);

                info!(
                    block_number = header.number,
                    sum_gas_used = sum_gas_used,
                    sum_tx = sum_tx,
                    sum_priority_fee = sum_priority_fee,
                    sum_base_fee = sum_base_fee,
                    "🧪 DRY-RUN: Would insert L2 header with calculated stats"
                );

                Ok(())
            }
            TaikoEvent::BatchProposed(wrapper) => {
                let batch = &wrapper.batch;
                info!(
                    batch_id = batch.meta.batchId,
                    last_block = batch.last_block_number(),
                    l1_tx_hash = %wrapper.l1_tx_hash,
                    tx_list_len = batch.txList.len(),
                    "🧪 DRY-RUN: Would process BatchProposed"
                );

                // Simulate cost calculation
                if let Some(cost) =
                    Self::fetch_transaction_cost(&self.extractor, wrapper.l1_tx_hash).await
                {
                    info!(
                        batch_id = batch.meta.batchId,
                        l1_data_cost = cost,
                        "🧪 DRY-RUN: Would insert L1 data cost"
                    );
                }

                info!(batch_id = batch.meta.batchId, "🧪 DRY-RUN: Would insert batch row");

                Ok(())
            }
            TaikoEvent::ForcedInclusionProcessed(wrapper) => {
                info!(
                    blob_hash = ?wrapper.event.forcedInclusion.blobHash,
                    "🧪 DRY-RUN: Would process ForcedInclusionProcessed"
                );

                info!(
                    blob_hash = ?wrapper.event.forcedInclusion.blobHash,
                    "🧪 DRY-RUN: Would insert forced inclusion record"
                );

                Ok(())
            }
            TaikoEvent::BatchesProved(wrapper) => {
                let proved = &wrapper.proved;
                info!(
                    batch_ids = ?proved.batch_ids_proved(),
                    l1_block_number = wrapper.l1_block_number,
                    l1_tx_hash = %wrapper.l1_tx_hash,
                    "🧪 DRY-RUN: Would process BatchesProved"
                );

                // Simulate cost calculation
                if let Some(cost) =
                    Self::fetch_transaction_cost(&self.extractor, wrapper.l1_tx_hash).await
                {
                    let cost_per_batch =
                        Self::average_cost_per_batch(cost, proved.batch_ids_proved().len());
                    info!(
                        batch_count = proved.batch_ids_proved().len(),
                        total_cost = cost,
                        cost_per_batch = cost_per_batch,
                        "🧪 DRY-RUN: Would insert prove costs"
                    );
                }

                info!(
                    batch_ids = ?proved.batch_ids_proved(),
                    "🧪 DRY-RUN: Would insert proved batch records"
                );

                Ok(())
            }
            TaikoEvent::BatchesVerified(wrapper) => {
                let verified = &wrapper.verified;
                info!(
                    batch_id = verified.batch_id,
                    l1_block_number = wrapper.l1_block_number,
                    l1_tx_hash = %wrapper.l1_tx_hash,
                    "🧪 DRY-RUN: Would process BatchesVerified"
                );

                // Simulate cost calculation
                if let Some(cost) =
                    Self::fetch_transaction_cost(&self.extractor, wrapper.l1_tx_hash).await
                {
                    info!(
                        batch_id = verified.batch_id,
                        verify_cost = cost,
                        "🧪 DRY-RUN: Would insert verify cost"
                    );
                }

                info!(
                    batch_id = verified.batch_id,
                    "🧪 DRY-RUN: Would insert verified batch record"
                );

                Ok(())
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
        let writer = match &self.clickhouse_writer {
            Some(w) => w,
            None => {
                // When database writes disabled, we still want to validate the preconf data logic
                if self.enable_db_writes {
                    return;
                }
                info!(
                    block_number = header.number,
                    "🧪 DRY-RUN: Validating preconf data processing without database writes"
                );
                // Continue validation but skip database writes
                return self.process_preconf_data_dry_run(header).await;
            }
        };

        // Get operator candidates for current epoch
        let opt_candidates = match self.extractor.get_operator_candidates_for_current_epoch().await
        {
            Ok(c) => {
                info!(
                    slot = header.slot,
                    block = header.number,
                    candidates = ?c,
                    candidates_count = c.len(),
                    "Successfully retrieved operator candidates"
                );
                Some(c)
            }
            Err(e) => {
                error!(
                    slot = header.slot,
                    block = header.number,
                    err = %e,
                    "Failed picking operator candidates"
                );
                None
            }
        };
        let candidates = opt_candidates.unwrap_or_else(Vec::new);

        // Get current operator for epoch
        let opt_current_operator = match self.extractor.get_operator_for_current_epoch().await {
            Ok(op) => {
                info!(
                    block = header.number,
                    operator = ?op,
                    "Current operator for epoch"
                );
                Some(op)
            }
            Err(e) => {
                error!(block = header.number, err = %e, "get_operator_for_current_epoch failed");
                None
            }
        };

        // Get next operator for epoch
        let opt_next_operator = match self.extractor.get_operator_for_next_epoch().await {
            Ok(op) => {
                info!(
                    block = header.number,
                    operator = ?op,
                    "Next operator for epoch"
                );
                Some(op)
            }
            Err(e) => {
                error!(block = header.number, err = %e, "get_operator_for_next_epoch failed");
                None
            }
        };

        // Insert preconf data if we have at least one operator
        if opt_current_operator.is_some() || opt_next_operator.is_some() {
            if let Err(e) = writer
                .insert_preconf_data(
                    header.slot,
                    candidates,
                    opt_current_operator,
                    opt_next_operator,
                )
                .await
            {
                error!(slot = header.slot, err = %e, "Failed to insert preconf data");
            } else {
                info!(slot = header.slot, "Inserted preconf data for slot");
            }
        } else {
            info!(
                slot = header.slot,
                "Skipping preconf data insertion due to errors fetching operator data"
            );
        }
    }

    async fn process_preconf_data_dry_run(&self, header: &primitives::headers::L1Header) {
        // Get operator candidates for current epoch (for validation)
        let opt_candidates = match self.extractor.get_operator_candidates_for_current_epoch().await
        {
            Ok(c) => {
                info!(
                    slot = header.slot,
                    block = header.number,
                    candidates = ?c,
                    candidates_count = c.len(),
                    "🧪 DRY-RUN: Retrieved operator candidates"
                );
                Some(c)
            }
            Err(e) => {
                warn!(
                    slot = header.slot,
                    block = header.number,
                    err = %e,
                    "🧪 DRY-RUN: Failed picking operator candidates"
                );
                None
            }
        };
        let candidates = opt_candidates.unwrap_or_else(Vec::new);

        // Get current operator for epoch (for validation)
        let opt_current_operator = match self.extractor.get_operator_for_current_epoch().await {
            Ok(op) => {
                info!(
                    block = header.number,
                    operator = ?op,
                    "🧪 DRY-RUN: Current operator for epoch"
                );
                Some(op)
            }
            Err(e) => {
                warn!(block = header.number, err = %e, "🧪 DRY-RUN: get_operator_for_current_epoch failed");
                None
            }
        };

        // Get next operator for epoch (for validation)
        let opt_next_operator = match self.extractor.get_operator_for_next_epoch().await {
            Ok(op) => {
                info!(
                    block = header.number,
                    operator = ?op,
                    "🧪 DRY-RUN: Next operator for epoch"
                );
                Some(op)
            }
            Err(e) => {
                warn!(block = header.number, err = %e, "🧪 DRY-RUN: get_operator_for_next_epoch failed");
                None
            }
        };

        // Simulate database insertion
        if opt_current_operator.is_some() || opt_next_operator.is_some() {
            info!(
                slot = header.slot,
                candidate_count = candidates.len(),
                has_current_op = opt_current_operator.is_some(),
                has_next_op = opt_next_operator.is_some(),
                "🧪 DRY-RUN: Would insert preconf data"
            );
        } else {
            info!(
                slot = header.slot,
                "🧪 DRY-RUN: Would skip preconf data insertion due to missing operators"
            );
        }
    }

    async fn process_reorg_detection(&mut self, header: &primitives::headers::L2Header) {
        let writer = match &self.clickhouse_writer {
            Some(w) => w,
            None => return,
        };

        let old_head = self.reorg_detector.head_number();
        let reorg_result = self.reorg_detector.on_new_block_with_hash(header.number, header.hash);

        // Update last L2 header tracking
        self.last_l2_header = Some((header.number, header.beneficiary));

        if let Some((depth, orphaned_hash)) = reorg_result {
            warn!(
                old_head = old_head,
                new_head = header.number,
                depth = depth,
                orphaned_hash = ?orphaned_hash,
                "L2 reorg detected"
            );

            // Handle orphaned hash from one-block reorg
            if let Some(hash) = orphaned_hash {
                Self::insert_orphaned_hash(writer, hash, header.number).await;
            }

            // Handle orphaned blocks from traditional reorg
            if depth > 0 {
                Self::handle_traditional_reorg_orphans(
                    writer,
                    &self.clickhouse_reader,
                    old_head,
                    header.number,
                    depth,
                )
                .await;
            }

            // Check if we need to process L2 reorg with previous sequencer
            if let Some((prev_block_number, prev_sequencer)) = self.last_l2_header {
                if prev_sequencer != header.beneficiary {
                    info!(
                        prev_block = prev_block_number,
                        new_block = header.number,
                        prev_sequencer = ?prev_sequencer,
                        new_sequencer = ?header.beneficiary,
                        "L2 reorg with sequencer change detected"
                    );

                    // Insert L2 reorg record
                    if let Err(e) = writer
                        .insert_l2_reorg(header.number, depth, prev_sequencer, header.beneficiary)
                        .await
                    {
                        error!(
                            block_number = header.number,
                            err = %e,
                            "Failed to insert L2 reorg record"
                        );
                    } else {
                        info!(
                            block_number = header.number,
                            depth = depth,
                            "Inserted L2 reorg record"
                        );
                    }
                }
            }
        }
    }

    async fn insert_orphaned_hash(
        writer: &ClickhouseWriter,
        hash: alloy_primitives::B256,
        block_number: u64,
    ) {
        if let Err(e) =
            writer.insert_orphaned_hashes(&[(HashBytes::from(hash), block_number)]).await
        {
            error!(block_number, orphaned_hash = ?hash, err = %e, "Failed to insert orphaned hash");
        } else {
            info!(block_number, orphaned_hash = ?hash, "Inserted orphaned hash");
        }
    }

    async fn handle_traditional_reorg_orphans(
        writer: &ClickhouseWriter,
        clickhouse_reader: &Option<ClickhouseReader>,
        old_head: u64,
        new_head: u64,
        depth: u16,
    ) {
        let orphaned_block_numbers =
            Self::calculate_orphaned_blocks(old_head, new_head, depth.into());
        if orphaned_block_numbers.is_empty() {
            return;
        }

        let Some(reader) = clickhouse_reader else {
            return;
        };

        match reader.get_latest_hashes_for_blocks(&orphaned_block_numbers).await {
            Ok(orphaned_hashes) if !orphaned_hashes.is_empty() => {
                if let Err(e) = writer.insert_orphaned_hashes(&orphaned_hashes).await {
                    error!(count = orphaned_hashes.len(), err = %e, "Failed to insert orphaned hashes");
                } else {
                    info!(count = orphaned_hashes.len(), "Inserted orphaned hashes for reorg");
                }
            }
            Ok(_) => {} // No orphaned hashes found
            Err(e) => error!(err = %e, "Failed to fetch orphaned hashes"),
        }
    }

    fn calculate_orphaned_blocks(old_head: u64, new_head: u64, _depth: u32) -> Vec<u64> {
        // Orphaned blocks are from new_head+1 to old_head (inclusive)
        if new_head >= old_head {
            // No orphaned blocks if new_head is >= old_head
            return Vec::new();
        }
        let orphaned_start = new_head + 1;
        let orphaned_end = old_head + 1; // +1 because range is exclusive at end
        (orphaned_start..orphaned_end).collect()
    }

    async fn insert_l2_header_with_stats(&self, header: &primitives::headers::L2Header) {
        let writer = match &self.clickhouse_writer {
            Some(w) => w,
            None => return,
        };

        let (sum_gas_used, sum_tx, sum_priority_fee) = self.extractor
            .get_l2_block_stats(alloy_primitives::B256::from(*header.hash), header.base_fee_per_gas)
            .await
            .unwrap_or_else(|e| {
                error!(header_number = header.number, err = %e, "Failed to get L2 block stats, using defaults");
                (0, 0, 0)
            });

        let sum_base_fee = sum_gas_used.saturating_mul(header.base_fee_per_gas as u128);

        let event = L2HeadEvent {
            l2_block_number: header.number,
            block_hash: HashBytes(*header.hash),
            block_ts: header.timestamp,
            sum_gas_used,
            sum_tx,
            sum_priority_fee,
            sum_base_fee,
            sequencer: AddressBytes(header.beneficiary.into_array()),
        };

        if let Err(e) = writer.insert_l2_header(&event).await {
            error!(header_number = header.number, err = %e, "Failed to insert L2 header");
        } else {
            info!(header_number = header.number, "Inserted L2 header with stats");
        }
    }

    /// Start monitoring tasks if enabled
    /// Note: Monitor implementation is deferred to future development
    async fn start_monitors(&self) -> Vec<tokio::task::JoinHandle<()>> {
        info!(
            "Monitors disabled in unified mode - monitoring functionality to be implemented in future versions"
        );
        Vec::new()
    }

    /// Start the gap detection and backfill task
    async fn start_gap_detection_task(&self) -> Option<tokio::task::JoinHandle<()>> {
        // Only start gap detection if we have a reader
        let reader = self.clickhouse_reader.as_ref()?.clone();
        let writer = self.clickhouse_writer.as_ref()?.clone();
        let extractor = self.extractor.clone();
        let inbox_address = self.inbox_address;
        let taiko_wrapper_address = self.taiko_wrapper_address;
        let enable_db_writes = self.enable_db_writes;

        info!("Starting gap detection task");

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                interval.tick().await;

                if let Err(e) = Self::run_gap_detection(
                    &reader,
                    &writer,
                    &extractor,
                    inbox_address,
                    taiko_wrapper_address,
                    enable_db_writes,
                )
                .await
                {
                    error!(err = %e, "Gap detection failed");
                } else {
                    info!("Gap detection cycle completed");
                }
            }
        });

        Some(handle)
    }

    /// Run a single cycle of gap detection and backfill
    async fn run_gap_detection(
        reader: &ClickhouseReader,
        writer: &ClickhouseWriter,
        extractor: &Extractor,
        inbox_address: Address,
        taiko_wrapper_address: Address,
        enable_db_writes: bool,
    ) -> Result<()> {
        // Get current blockchain state
        let latest_l1_rpc = extractor
            .get_l1_latest_block_number()
            .await
            .map_err(|e| eyre::eyre!("Failed to get latest L1 block: {}", e))?;
        let latest_l2_rpc = extractor
            .get_l2_latest_block_number()
            .await
            .map_err(|e| eyre::eyre!("Failed to get latest L2 block: {}", e))?;

        // Get database state
        let latest_l1_db = reader.get_latest_l1_block().await?.unwrap_or(0);
        let latest_l2_db = reader.get_latest_l2_block().await?.unwrap_or(0);

        info!(
            latest_l1_rpc = latest_l1_rpc,
            latest_l1_db = latest_l1_db,
            latest_l2_rpc = latest_l2_rpc,
            latest_l2_db = latest_l2_db,
            "Gap detection: blockchain vs database state"
        );

        // Only backfill finalized data (5+ blocks old)
        const FINALIZATION_BUFFER: u64 = 5;
        let l1_backfill_end = latest_l1_rpc.saturating_sub(FINALIZATION_BUFFER);
        let l2_backfill_end = latest_l2_rpc.saturating_sub(FINALIZATION_BUFFER);

        // Find and backfill L1 gaps
        if latest_l1_db < l1_backfill_end {
            let l1_gaps = reader.find_missing_l1_blocks(latest_l1_db + 1, l1_backfill_end).await?;
            if !l1_gaps.is_empty() {
                if enable_db_writes {
                    info!(gaps = l1_gaps.len(), "Found L1 gaps to backfill: {:?}", l1_gaps);
                    Self::backfill_l1_blocks(
                        writer,
                        extractor,
                        l1_gaps,
                        inbox_address,
                        taiko_wrapper_address,
                        enable_db_writes,
                    )
                    .await?;
                } else {
                    info!(
                        gaps = l1_gaps.len(),
                        "🧪 DRY-RUN: Would backfill L1 gaps: {:?}", l1_gaps
                    );
                }
            }
        }

        // Find and backfill L2 gaps
        if latest_l2_db < l2_backfill_end {
            let l2_gaps = reader.find_missing_l2_blocks(latest_l2_db + 1, l2_backfill_end).await?;
            if !l2_gaps.is_empty() {
                if enable_db_writes {
                    info!(gaps = l2_gaps.len(), "Found L2 gaps to backfill: {:?}", l2_gaps);
                    Self::backfill_l2_blocks(writer, extractor, l2_gaps, enable_db_writes).await?;
                } else {
                    info!(
                        gaps = l2_gaps.len(),
                        "🧪 DRY-RUN: Would backfill L2 gaps: {:?}", l2_gaps
                    );
                }
            }
        }

        Ok(())
    }

    /// Backfill missing L1 blocks and extract all Taiko events from those blocks
    async fn backfill_l1_blocks(
        writer: &ClickhouseWriter,
        extractor: &Extractor,
        block_numbers: Vec<u64>,
        inbox_address: Address,
        taiko_wrapper_address: Address,
        enable_db_writes: bool,
    ) -> Result<()> {
        for block_number in block_numbers {
            match extractor.get_l1_block_by_number(block_number).await {
                Ok(block) => {
                    // Insert L1 header
                    let header = primitives::headers::L1Header {
                        number: block.header.number,
                        hash: block.header.hash,
                        slot: block.header.timestamp, // Using timestamp as slot for now
                        timestamp: block.header.timestamp,
                    };

                    if enable_db_writes {
                        if let Err(e) = writer.insert_l1_header(&header).await {
                            error!(block_number = block_number, err = %e, "Failed to backfill L1 header");
                            continue;
                        }
                    } else {
                        info!(
                            block_number = block_number,
                            hash = %header.hash,
                            "🧪 DRY-RUN: Would insert L1 header"
                        );
                    }

                    // Process preconf data - skip for backfill since we don't have the driver
                    // instance
                    info!(
                        block_number = header.number,
                        "Preconf data processing skipped during backfill"
                    );

                    // Process all Taiko events from this L1 block
                    Self::process_l1_block_taiko_events(
                        writer,
                        extractor,
                        &block,
                        inbox_address,
                        taiko_wrapper_address,
                        enable_db_writes,
                    )
                    .await?;

                    if enable_db_writes {
                        info!(
                            block_number = block_number,
                            "Successfully backfilled L1 block with events"
                        );
                    } else {
                        info!(
                            block_number = block_number,
                            "🧪 DRY-RUN: Would complete L1 block backfill with events"
                        );
                    }
                }
                Err(e) => {
                    warn!(block_number = block_number, err = %e, "Could not fetch L1 block for backfill");
                }
            }
        }
        Ok(())
    }

    /// Process all Taiko events found in an L1 block during backfill
    async fn process_l1_block_taiko_events(
        writer: &ClickhouseWriter,
        extractor: &Extractor,
        block: &alloy_rpc_types_eth::Block,
        inbox_address: Address,
        taiko_wrapper_address: Address,
        enable_db_writes: bool,
    ) -> Result<()> {
        #[allow(unused_imports)]
        use alloy_sol_types::SolEvent;
        use chainio::{
            BatchesVerified,
            ITaikoInbox::{BatchProposed, BatchesProved, BatchesVerified as InboxBatchesVerified},
            taiko::wrapper::ITaikoWrapper::ForcedInclusionProcessed,
        };
        use messages::{
            BatchProposedWrapper, BatchesProvedWrapper, BatchesVerifiedWrapper,
            ForcedInclusionProcessedWrapper,
        };

        let block_number = block.header.number;
        let mut events_found = 0;

        info!(
            block_number = block_number,
            tx_count = block.transactions.len(),
            "Processing L1 block for Taiko events during backfill"
        );

        // Process all transactions in the block to find Taiko events
        for tx_hash in block.transactions.hashes() {
            // Get transaction receipt to access logs
            match extractor.get_receipt(tx_hash).await {
                Ok(receipt) => {
                    for log in receipt.logs() {
                        // Skip removed logs (shouldn't happen in backfill but be safe)
                        if log.removed {
                            continue;
                        }

                        // Process events based on contract address
                        if log.address() == inbox_address {
                            // Try to decode BatchProposed
                            if let Ok(decoded) = log.log_decode::<BatchProposed>() {
                                info!(
                                    block_number = block_number,
                                    tx_hash = %tx_hash,
                                    "Found BatchProposed event in backfill"
                                );
                                let wrapper = BatchProposedWrapper::from((
                                    decoded.data().clone(),
                                    tx_hash,
                                    false, // not reorged
                                ));
                                Self::handle_batch_proposed_event_during_backfill(
                                    writer,
                                    extractor,
                                    wrapper,
                                    enable_db_writes,
                                )
                                .await?;
                                events_found += 1;
                                continue;
                            }

                            // Try to decode BatchesProved
                            if let Ok(decoded) = log.log_decode::<BatchesProved>() {
                                info!(
                                    block_number = block_number,
                                    tx_hash = %tx_hash,
                                    "Found BatchesProved event in backfill"
                                );
                                let wrapper = BatchesProvedWrapper::from((
                                    decoded.data().clone(),
                                    block_number,
                                    tx_hash,
                                    false, // not reorged
                                ));
                                Self::handle_batches_proved_event_during_backfill(
                                    writer,
                                    extractor,
                                    wrapper,
                                    enable_db_writes,
                                )
                                .await?;
                                events_found += 1;
                                continue;
                            }

                            // Try to decode BatchesVerified
                            if let Ok(decoded) = log.log_decode::<InboxBatchesVerified>() {
                                info!(
                                    block_number = block_number,
                                    tx_hash = %tx_hash,
                                    "Found BatchesVerified event in backfill"
                                );
                                let data = decoded.data();
                                let mut block_hash = [0u8; 32];
                                block_hash.copy_from_slice(data.blockHash.as_slice());
                                let verified =
                                    BatchesVerified { batch_id: data.batchId, block_hash };
                                let wrapper = BatchesVerifiedWrapper::from((
                                    verified,
                                    block_number,
                                    tx_hash,
                                    false, // not reorged
                                ));
                                Self::handle_batches_verified_event_during_backfill(
                                    writer,
                                    extractor,
                                    wrapper,
                                    enable_db_writes,
                                )
                                .await?;
                                events_found += 1;
                            }
                        } else if log.address() == taiko_wrapper_address {
                            // Try to decode ForcedInclusionProcessed
                            if let Ok(decoded) = log.log_decode::<ForcedInclusionProcessed>() {
                                info!(
                                    block_number = block_number,
                                    tx_hash = %tx_hash,
                                    "Found ForcedInclusionProcessed event in backfill"
                                );
                                let wrapper = ForcedInclusionProcessedWrapper::from((
                                    decoded.data().clone(),
                                    false, // not reorged
                                ));
                                Self::handle_forced_inclusion_event_during_backfill(
                                    writer,
                                    wrapper,
                                    enable_db_writes,
                                )
                                .await?;
                                events_found += 1;
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        block_number = block_number,
                        tx_hash = %tx_hash,
                        err = %e,
                        "Failed to get receipt for transaction during L1 backfill"
                    );
                }
            }
        }

        info!(
            block_number = block_number,
            events_found = events_found,
            "Completed L1 block Taiko event extraction during backfill"
        );
        Ok(())
    }

    // Event handlers for backfill - reuse exact same logic as live processing
    async fn handle_batch_proposed_event_during_backfill(
        writer: &ClickhouseWriter,
        extractor: &Extractor,
        wrapper: BatchProposedWrapper,
        enable_db_writes: bool,
    ) -> Result<()> {
        let batch = &wrapper.batch;
        let l1_tx_hash = wrapper.l1_tx_hash;

        // Insert batch with error handling
        if enable_db_writes {
            Self::with_db_error_context(
                writer.insert_batch(batch, l1_tx_hash),
                "insert batch",
                format!("batch_last_block={:?}", batch.last_block_number()),
            )
            .await?;
        } else {
            info!(
                batch_id = batch.meta.batchId,
                last_block = batch.last_block_number(),
                l1_tx_hash = %l1_tx_hash,
                "🧪 DRY-RUN: Would insert batch"
            );
        }

        // Calculate and insert L1 data cost
        if let Some(cost) = Self::fetch_transaction_cost(extractor, l1_tx_hash).await {
            if enable_db_writes {
                Self::with_db_error_context(
                    writer.insert_l1_data_cost(batch.info.proposedIn, batch.meta.batchId, cost),
                    "insert L1 data cost",
                    format!("l1_block_number={}, tx_hash={:?}", batch.info.proposedIn, l1_tx_hash),
                )
                .await?;
            } else {
                info!(
                    l1_block_number = batch.info.proposedIn,
                    batch_id = batch.meta.batchId,
                    cost = cost,
                    "🧪 DRY-RUN: Would insert L1 data cost"
                );
            }
        }
        Ok(())
    }

    async fn handle_batches_proved_event_during_backfill(
        writer: &ClickhouseWriter,
        extractor: &Extractor,
        wrapper: BatchesProvedWrapper,
        enable_db_writes: bool,
    ) -> Result<()> {
        let proved = &wrapper.proved;
        let l1_block_number = wrapper.l1_block_number;
        let l1_tx_hash = wrapper.l1_tx_hash;

        // Insert proved batch
        if enable_db_writes {
            Self::with_db_error_context(
                writer.insert_proved_batch(proved, l1_block_number),
                "insert proved batch",
                format!("batch_ids={:?}", proved.batch_ids_proved()),
            )
            .await?;
        } else {
            info!(
                batch_ids = ?proved.batch_ids_proved(),
                l1_block_number = l1_block_number,
                "🧪 DRY-RUN: Would insert proved batch"
            );
        }

        // Calculate and insert prove costs for each batch
        if let Some(cost) = Self::fetch_transaction_cost(extractor, l1_tx_hash).await {
            let cost_per_batch =
                Self::average_cost_per_batch(cost, proved.batch_ids_proved().len());

            if enable_db_writes {
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
            } else {
                info!(
                    l1_block_number = l1_block_number,
                    batch_ids = ?proved.batch_ids_proved(),
                    cost_per_batch = cost_per_batch,
                    "🧪 DRY-RUN: Would insert prove costs for {} batches",
                    proved.batch_ids_proved().len()
                );
            }
        }
        Ok(())
    }

    async fn handle_batches_verified_event_during_backfill(
        writer: &ClickhouseWriter,
        extractor: &Extractor,
        wrapper: BatchesVerifiedWrapper,
        enable_db_writes: bool,
    ) -> Result<()> {
        let verified = &wrapper.verified;
        let l1_block_number = wrapper.l1_block_number;
        let l1_tx_hash = wrapper.l1_tx_hash;

        // Insert verified batch
        if enable_db_writes {
            Self::with_db_error_context(
                writer.insert_verified_batch(verified, l1_block_number),
                "insert verified batch",
                format!("batch_id={}", verified.batch_id),
            )
            .await?;
        } else {
            info!(
                batch_id = verified.batch_id,
                l1_block_number = l1_block_number,
                "🧪 DRY-RUN: Would insert verified batch"
            );
        }

        // Calculate and insert verify cost
        if let Some(cost) = Self::fetch_transaction_cost(extractor, l1_tx_hash).await {
            if enable_db_writes {
                Self::with_db_error_context(
                    writer.insert_verify_cost(l1_block_number, verified.batch_id, cost),
                    "insert verify cost",
                    format!(
                        "l1_block_number={}, batch_id={}, tx_hash={:?}",
                        l1_block_number, verified.batch_id, l1_tx_hash
                    ),
                )
                .await?;
            } else {
                info!(
                    l1_block_number = l1_block_number,
                    batch_id = verified.batch_id,
                    cost = cost,
                    "🧪 DRY-RUN: Would insert verify cost"
                );
            }
        }
        Ok(())
    }

    async fn handle_forced_inclusion_event_during_backfill(
        writer: &ClickhouseWriter,
        wrapper: ForcedInclusionProcessedWrapper,
        enable_db_writes: bool,
    ) -> Result<()> {
        let event = &wrapper.event;

        if enable_db_writes {
            Self::with_db_error_context(
                writer.insert_forced_inclusion(event),
                "insert forced inclusion",
                format!("blob_hash={:?}", event.forcedInclusion.blobHash),
            )
            .await?;
        } else {
            info!(
                blob_hash = ?event.forcedInclusion.blobHash,
                "🧪 DRY-RUN: Would insert forced inclusion"
            );
        }

        Ok(())
    }

    /// Backfill missing L2 blocks using exact same logic as live processing
    async fn backfill_l2_blocks(
        writer: &ClickhouseWriter,
        extractor: &Extractor,
        block_numbers: Vec<u64>,
        enable_db_writes: bool,
    ) -> Result<()> {
        for block_number in block_numbers {
            match extractor.get_l2_block_by_number(block_number).await {
                Ok(block) => {
                    let header = primitives::headers::L2Header {
                        number: block.header.number,
                        hash: block.header.hash,
                        parent_hash: block.header.parent_hash,
                        timestamp: block.header.timestamp,
                        gas_used: block.header.gas_used,
                        beneficiary: block.header.beneficiary,
                        base_fee_per_gas: block.header.base_fee_per_gas.unwrap_or(0),
                    };

                    // Use same stats calculation as processor
                    let (sum_gas_used, sum_tx, sum_priority_fee) = extractor
                        .get_l2_block_stats(alloy_primitives::B256::from(*header.hash), header.base_fee_per_gas)
                        .await
                        .unwrap_or_else(|e| {
                            error!(header_number = header.number, err = %e, "Failed to get L2 block stats for backfill, using defaults");
                            (0, 0, 0)
                        });

                    let sum_base_fee = sum_gas_used.saturating_mul(header.base_fee_per_gas as u128);

                    let event = L2HeadEvent {
                        l2_block_number: header.number,
                        block_hash: HashBytes(*header.hash),
                        block_ts: header.timestamp,
                        sum_gas_used,
                        sum_tx,
                        sum_priority_fee,
                        sum_base_fee,
                        sequencer: AddressBytes(header.beneficiary.into_array()),
                    };

                    if enable_db_writes {
                        if let Err(e) = writer.insert_l2_header(&event).await {
                            error!(block_number = block_number, err = %e, "Failed to backfill L2 block");
                        } else {
                            info!(block_number = block_number, "Successfully backfilled L2 block");
                        }
                    } else {
                        info!(
                            block_number = block_number,
                            gas_used = event.sum_gas_used,
                            tx_count = event.sum_tx,
                            "🧪 DRY-RUN: Would insert L2 header with stats"
                        );
                    }
                }
                Err(e) => {
                    warn!(block_number = block_number, err = %e, "Could not fetch L2 block for backfill");
                }
            }
        }
        Ok(())
    }
}
