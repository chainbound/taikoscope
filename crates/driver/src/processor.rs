//! Taikoscope Processor Driver

use std::{collections::VecDeque, time::Duration};

use alloy_primitives::{Address, B256, BlockHash};
use clickhouse::{AddressBytes, ClickhouseReader, ClickhouseWriter, HashBytes, L2HeadEvent};
use config::Opts;
use extractor::{Extractor, ReorgDetector};
use eyre::{Context, Result};
use incident::{
    BatchProofTimeoutMonitor, InstatusL1Monitor, InstatusMonitor, Monitor,
    client::Client as IncidentClient,
    monitor::{BatchVerifyTimeoutMonitor, spawn_public_rpc_monitor},
};
use nats_utils::TaikoEvent;
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tracing::info;
use url::Url;

use crate::subscription::subscribe_with_retry;

/// An EPOCH is a series of 32 slots.
const EPOCH_SLOTS: u64 = 32;

/// Driver for the processor service that consumes NATS events and writes to `ClickHouse`
#[derive(Debug)]
pub struct ProcessorDriver {
    nats_client: async_nats::Client,
    clickhouse_writer: Option<ClickhouseWriter>,
    clickhouse_reader: Option<ClickhouseReader>,
    extractor: Extractor,
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
    nats_stream_config: config::NatsStreamOpts,
    processed_l2_headers: VecDeque<BlockHash>,
}

/// Components extracted from `ProcessorDriver` for message processing
struct ProcessorComponents {
    clickhouse_writer: Option<ClickhouseWriter>,
    clickhouse_reader: Option<ClickhouseReader>,
    extractor: Extractor,
    reorg_detector: ReorgDetector,
    last_l2_header: Option<(u64, Address)>,
    enable_db_writes: bool,
    processed_l2_headers: VecDeque<BlockHash>,
}

impl ProcessorDriver {
    /// Create a new processor driver with the given configuration
    pub async fn new(opts: Opts) -> Result<Self> {
        info!("Initializing processor driver");

        // verify monitoring configuration before doing any heavy work
        if opts.instatus.monitors_enabled && !opts.instatus.enabled() {
            return Err(eyre::eyre!(
                "Instatus configuration missing; set the INSTATUS_* environment variables"
            ));
        }

        if !opts.instatus.monitors_enabled {
            info!("Instatus monitors disabled; no incidents will be reported");
        }

        let nats_client = async_nats::connect(&opts.nats_url)
            .await
            .wrap_err_with(|| format!("failed to connect to NATS at {}", opts.nats_url))?;
        info!("Connected to NATS server at {}", opts.nats_url);

        // Initialize extractor for L2 block statistics and transaction cost analysis
        let extractor = Extractor::new(
            opts.rpc.l1_url.clone(),
            opts.rpc.l2_url.clone(),
            opts.taiko_addresses.inbox_address,
            opts.taiko_addresses.preconf_whitelist_address,
            opts.taiko_addresses.taiko_wrapper_address,
        )
        .await?;

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
            nats_client,
            clickhouse_writer,
            clickhouse_reader,
            extractor,
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
            nats_stream_config: opts.nats_stream,
            processed_l2_headers: VecDeque::new(),
        })
    }

    /// Start the processor event loop, consuming NATS events and processing them
    pub async fn start(self) -> Result<()> {
        self.start_with_shutdown(None).await
    }

    /// Start the processor event loop with graceful shutdown support
    pub async fn start_with_shutdown(
        self,
        shutdown_rx: Option<broadcast::Receiver<()>>,
    ) -> Result<()> {
        info!("Starting processor event loop");

        if self.enable_db_writes {
            info!("Database writes ENABLED - events will be processed and stored");
        } else {
            info!("Database writes DISABLED - events will be logged and dropped");
        }

        // Spawn monitors before starting the event loop
        self.spawn_monitors();

        // Setup NATS infrastructure with retry
        let consumer = self.setup_nats_infrastructure().await?;

        // Extract components for processing
        let ProcessorComponents {
            clickhouse_writer,
            clickhouse_reader,
            extractor,
            mut reorg_detector,
            mut last_l2_header,
            enable_db_writes,
            mut processed_l2_headers,
        } = self.into_components();

        // Run the message processing loop (as static reference since self is consumed)
        Self::run_message_processing_loop_static(
            consumer,
            clickhouse_writer,
            clickhouse_reader,
            extractor,
            &mut reorg_detector,
            &mut last_l2_header,
            enable_db_writes,
            &mut processed_l2_headers,
            shutdown_rx,
        )
        .await
    }

    /// Extract components from self for processing
    fn into_components(self) -> ProcessorComponents {
        ProcessorComponents {
            clickhouse_writer: self.clickhouse_writer,
            clickhouse_reader: self.clickhouse_reader,
            extractor: self.extractor,
            reorg_detector: self.reorg_detector,
            last_l2_header: self.last_l2_header,
            enable_db_writes: self.enable_db_writes,
            processed_l2_headers: self.processed_l2_headers,
        }
    }

    /// Setup NATS infrastructure with retry logic for robustness
    async fn setup_nats_infrastructure(
        &self,
    ) -> Result<
        async_nats::jetstream::consumer::Consumer<async_nats::jetstream::consumer::pull::Config>,
    > {
        let jetstream = async_nats::jetstream::new(self.nats_client.clone());
        let nats_stream_config = &self.nats_stream_config;

        // Health check: Verify NATS connection is alive
        match jetstream.get_stream("taiko").await {
            Ok(_) => {
                info!("NATS connection health check passed - stream accessible");
            }
            Err(e) => {
                info!("NATS stream does not exist yet, will be created: {}", e);
            }
        }

        // Create stream with retry using our existing pattern
        let _stream = subscribe_with_retry(
            || async {
                jetstream
                    .get_or_create_stream(async_nats::jetstream::stream::Config {
                        name: "taiko".to_owned(),
                        subjects: vec!["taiko.events".to_owned()],
                        duplicate_window: nats_stream_config.get_duplicate_window(),
                        storage: nats_stream_config.get_storage_type(),
                        retention: nats_stream_config.get_retention_policy(),
                        ..Default::default()
                    })
                    .await
                    .map_err(|e| eyre::eyre!("NATS stream creation failed: {}", e))
            },
            "NATS stream creation",
        )
        .await;

        info!(
            duplicate_window_secs = nats_stream_config.duplicate_window_secs,
            storage_type = nats_stream_config.storage_type,
            retention_policy = nats_stream_config.retention_policy,
            "Successfully created NATS stream with configuration"
        );

        // Create consumer with retry - CRITICAL: Use durable consumer to avoid reprocessing
        let consumer = subscribe_with_retry(
            || async {
                jetstream
                    .create_consumer_on_stream(
                        async_nats::jetstream::consumer::pull::Config {
                            durable_name: Some("processor".to_owned()),
                            // IMPORTANT: Explicit ack mode to prevent duplicate processing
                            ack_policy: async_nats::jetstream::consumer::AckPolicy::Explicit,
                            // Ensure we don't redeliver messages too quickly
                            max_deliver: 3,
                            ack_wait: Duration::from_secs(30),
                            ..Default::default()
                        },
                        "taiko",
                    )
                    .await
                    .map_err(|e| eyre::eyre!("NATS consumer creation failed: {}", e))
            },
            "NATS consumer creation",
        )
        .await;

        info!("Successfully created durable NATS consumer 'processor' with explicit ACK policy");
        Ok(consumer)
    }

    /// Run the main message processing loop with reconnection handling
    #[allow(clippy::too_many_arguments)]
    async fn run_message_processing_loop_static(
        consumer: async_nats::jetstream::consumer::Consumer<
            async_nats::jetstream::consumer::pull::Config,
        >,
        clickhouse_writer: Option<ClickhouseWriter>,
        clickhouse_reader: Option<ClickhouseReader>,
        extractor: Extractor,
        reorg_detector: &mut ReorgDetector,
        last_l2_header: &mut Option<(u64, Address)>,
        enable_db_writes: bool,
        processed_l2_headers: &mut VecDeque<BlockHash>,
        mut shutdown_rx: Option<broadcast::Receiver<()>>,
    ) -> Result<()> {
        info!("Starting message processing loop");

        loop {
            // Get message stream - recreate if it fails to avoid getting stuck
            let mut messages = match consumer.messages().await {
                Ok(msgs) => msgs,
                Err(e) => {
                    tracing::error!(err = %e, "Failed to get message stream, recreating consumer...");
                    // Note: Cannot recreate consumer in static context - this is a limitation
                    // In production, consider using a different architecture or connection pooling
                    tracing::error!("Cannot recreate consumer in static context, exiting loop");
                    return Err(eyre::eyre!("NATS consumer failed and cannot be recreated"));
                }
            };

            loop {
                let message_future = messages.next();
                let shutdown_future = async {
                    if let Some(ref mut rx) = shutdown_rx {
                        rx.recv().await.ok();
                    } else {
                        std::future::pending().await
                    }
                };

                tokio::select! {
                    message = message_future => {
                        match message {
                            Some(Ok(msg)) => {
                                // Process message with retry but careful ACK handling
                                Self::process_message_with_retry_static(
                                    &clickhouse_writer,
                                    &clickhouse_reader,
                                    &extractor,
                                    reorg_detector,
                                    last_l2_header,
                                    enable_db_writes,
                                    processed_l2_headers,
                                    &msg,
                                ).await;
                            }
                            Some(Err(e)) => {
                                tracing::error!(err = %e, "Error receiving message from NATS, will recreate stream");
                                break; // Break inner loop to recreate consumer
                            }
                            None => {
                                tracing::warn!("NATS message stream ended, recreating consumer");
                                break; // Break inner loop to recreate consumer
                            }
                        }
                    }
                    _ = shutdown_future => {
                        info!("Shutdown signal received, stopping message processing");
                        return Ok(());
                    }
                }
            }
        }
    }

    /// Process a single message with retry logic, ensuring proper ACK handling to prevent
    /// duplicates
    #[allow(clippy::too_many_arguments)]
    async fn process_message_with_retry_static(
        clickhouse_writer: &Option<ClickhouseWriter>,
        clickhouse_reader: &Option<ClickhouseReader>,
        extractor: &Extractor,
        reorg_detector: &mut ReorgDetector,
        last_l2_header: &mut Option<(u64, Address)>,
        enable_db_writes: bool,
        processed_l2_headers: &mut VecDeque<BlockHash>,
        msg: &async_nats::jetstream::Message,
    ) {
        let mut retries = 0;
        const MAX_RETRIES: u32 = 3;

        loop {
            match Self::process_message(
                clickhouse_writer,
                clickhouse_reader,
                extractor,
                reorg_detector,
                last_l2_header,
                enable_db_writes,
                processed_l2_headers,
                msg,
            )
            .await
            {
                Ok(()) => {
                    // CRITICAL: Only ACK after successful processing to prevent duplicates
                    if let Err(e) = msg.ack().await {
                        tracing::error!(err = %e, "Failed to ack message - may cause redelivery");
                    }
                    break;
                }
                Err(e) => {
                    if retries >= MAX_RETRIES {
                        tracing::error!(
                            err = %e,
                            retries = retries,
                            msg_subject = ?msg.subject,
                            "Failed to process message after all retries, nacking to prevent loss but may cause redelivery"
                        );
                        // NACK with delay to avoid immediate redelivery storms
                        if let Err(nack_err) = msg
                            .ack_with(async_nats::jetstream::AckKind::Nak(Some(
                                Duration::from_secs(30),
                            )))
                            .await
                        {
                            tracing::error!(err = %nack_err, "Failed to nack message");
                        }
                        break;
                    }
                    retries += 1;
                    tracing::warn!(
                        err = %e,
                        retry = retries,
                        max_retries = MAX_RETRIES,
                        "Failed to process message, retrying..."
                    );
                    // Exponential backoff
                    tokio::time::sleep(Duration::from_millis(100 * (1 << retries))).await;
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn process_message(
        clickhouse_writer: &Option<ClickhouseWriter>,
        clickhouse_reader: &Option<ClickhouseReader>,
        extractor: &Extractor,
        reorg_detector: &mut ReorgDetector,
        last_l2_header: &mut Option<(u64, Address)>,
        enable_db_writes: bool,
        processed_l2_headers: &mut VecDeque<BlockHash>,
        msg: &async_nats::jetstream::Message,
    ) -> Result<()> {
        let event: TaikoEvent = serde_json::from_slice(&msg.payload)?;

        if enable_db_writes {
            if let Some(writer) = clickhouse_writer {
                Self::process_event_with_db_write(
                    writer,
                    clickhouse_reader,
                    extractor,
                    reorg_detector,
                    last_l2_header,
                    processed_l2_headers,
                    event,
                )
                .await
            } else {
                tracing::error!("Database writes enabled but no writer available");
                Ok(())
            }
        } else {
            Self::process_event_log_and_drop(event, processed_l2_headers).await
        }
    }

    async fn process_event_with_db_write(
        writer: &ClickhouseWriter,
        clickhouse_reader: &Option<ClickhouseReader>,
        extractor: &Extractor,
        reorg_detector: &mut ReorgDetector,
        last_l2_header: &mut Option<(u64, Address)>,
        processed_l2_headers: &mut VecDeque<BlockHash>,
        event: TaikoEvent,
    ) -> Result<()> {
        match event {
            TaikoEvent::BatchProposed(wrapper) => {
                Self::handle_batch_proposed_event(writer, extractor, wrapper).await
            }
            TaikoEvent::ForcedInclusionProcessed(wrapper) => {
                Self::handle_forced_inclusion_event(writer, wrapper).await
            }
            TaikoEvent::BatchesProved(wrapper) => {
                Self::handle_batches_proved_event(writer, extractor, wrapper).await
            }
            TaikoEvent::BatchesVerified(wrapper) => {
                Self::handle_batches_verified_event(writer, extractor, wrapper).await
            }
            TaikoEvent::L1Header(header) => {
                Self::handle_l1_header_event(writer, extractor, header).await
            }
            TaikoEvent::L2Header(header) => {
                Self::handle_l2_header(
                    writer,
                    clickhouse_reader,
                    extractor,
                    reorg_detector,
                    last_l2_header,
                    processed_l2_headers,
                    header,
                )
                .await
            }
        }
    }

    async fn process_event_log_and_drop(
        event: TaikoEvent,
        processed_l2_headers: &mut VecDeque<BlockHash>,
    ) -> Result<()> {
        match event {
            TaikoEvent::BatchProposed(wrapper) => {
                info!(
                    batch_last_block = ?wrapper.batch.last_block_number(),
                    l1_tx_hash = ?wrapper.l1_tx_hash,
                    "Received BatchProposed event (dropped - DB writes disabled)"
                );
            }
            TaikoEvent::ForcedInclusionProcessed(wrapper) => {
                info!(
                    blob_hash = ?wrapper.event.forcedInclusion.blobHash,
                    "Received ForcedInclusionProcessed event (dropped - DB writes disabled)"
                );
            }
            TaikoEvent::BatchesProved(wrapper) => {
                info!(
                    batch_ids = ?wrapper.proved.batch_ids_proved(),
                    l1_block_number = wrapper.l1_block_number,
                    l1_tx_hash = ?wrapper.l1_tx_hash,
                    "Received BatchesProved event (dropped - DB writes disabled)"
                );
            }
            TaikoEvent::BatchesVerified(wrapper) => {
                info!(
                    batch_id = wrapper.verified.batch_id,
                    l1_block_number = wrapper.l1_block_number,
                    l1_tx_hash = ?wrapper.l1_tx_hash,
                    "Received BatchesVerified event (dropped - DB writes disabled)"
                );
            }
            TaikoEvent::L1Header(header) => {
                info!(
                    header_number = header.number,
                    "Received L1Header event (dropped - DB writes disabled)"
                );
            }
            TaikoEvent::L2Header(header) => {
                // Check if this header has already been processed to avoid duplicate processing
                if processed_l2_headers.contains(&header.hash) {
                    tracing::warn!(
                        header_number = header.number,
                        header_hash = ?header.hash,
                        "Duplicate L2Header detected from RPC, skipping processing (dropped - DB writes disabled)"
                    );
                    return Ok(());
                }

                // Add header to FIFO set, maintaining capacity of 1000
                processed_l2_headers.push_back(header.hash);
                if processed_l2_headers.len() > 1000 {
                    processed_l2_headers.pop_front();
                }

                info!(
                    header_number = header.number,
                    "Received L2Header event (dropped - DB writes disabled)"
                );
            }
        }
        Ok(())
    }

    /// Handle `BatchProposed` event with database insertion
    async fn handle_batch_proposed_event(
        writer: &ClickhouseWriter,
        extractor: &Extractor,
        wrapper: messages::BatchProposedWrapper,
    ) -> Result<()> {
        let batch = &wrapper.batch;
        let l1_tx_hash = wrapper.l1_tx_hash;

        // Insert batch with error handling
        Self::with_db_error_context(
            writer.insert_batch(batch, l1_tx_hash),
            "insert batch",
            format!("batch_last_block={:?}", batch.last_block_number()),
        )
        .await?;

        info!(last_block_number = ?batch.last_block_number(), "Inserted batch");

        // Calculate and insert L1 data cost
        if let Some(cost) = Self::fetch_transaction_cost(extractor, l1_tx_hash).await {
            Self::with_db_error_context(
                writer.insert_l1_data_cost(batch.info.proposedIn, batch.meta.batchId, cost),
                "insert L1 data cost",
                format!("l1_block_number={}, tx_hash={:?}", batch.info.proposedIn, l1_tx_hash),
            )
            .await?;

            info!(
                l1_block_number = batch.info.proposedIn,
                tx_hash = ?l1_tx_hash,
                cost,
                "Inserted L1 data cost"
            );
        }

        Ok(())
    }

    /// Handle `ForcedInclusionProcessed` event with database insertion
    async fn handle_forced_inclusion_event(
        writer: &ClickhouseWriter,
        wrapper: messages::ForcedInclusionProcessedWrapper,
    ) -> Result<()> {
        let event = &wrapper.event;

        Self::with_db_error_context(
            writer.insert_forced_inclusion(event),
            "insert forced inclusion",
            format!("blob_hash={:?}", event.forcedInclusion.blobHash),
        )
        .await?;

        info!(blob_hash = ?event.forcedInclusion.blobHash, "Inserted forced inclusion");
        Ok(())
    }

    /// Handle `BatchesProved` event with database insertion and cost calculation
    async fn handle_batches_proved_event(
        writer: &ClickhouseWriter,
        extractor: &Extractor,
        wrapper: messages::BatchesProvedWrapper,
    ) -> Result<()> {
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

        info!(batch_ids = ?proved.batch_ids_proved(), "Inserted proved batch");

        // Calculate and insert prove costs for each batch
        if let Some(cost) = Self::fetch_transaction_cost(extractor, l1_tx_hash).await {
            let cost_per_batch = average_cost_per_batch(cost, proved.batch_ids_proved().len());

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

                info!(
                    l1_block_number,
                    batch_id,
                    tx_hash = ?l1_tx_hash,
                    cost = cost_per_batch,
                    "Inserted prove cost"
                );
            }
        }

        Ok(())
    }

    /// Handle `BatchesVerified` event with database insertion and cost calculation
    async fn handle_batches_verified_event(
        writer: &ClickhouseWriter,
        extractor: &Extractor,
        wrapper: messages::BatchesVerifiedWrapper,
    ) -> Result<()> {
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

        info!(batch_id = verified.batch_id, "Inserted verified batch");

        // Calculate and insert verify cost
        if let Some(cost) = Self::fetch_transaction_cost(extractor, l1_tx_hash).await {
            Self::with_db_error_context(
                writer.insert_verify_cost(l1_block_number, verified.batch_id, cost),
                "insert verify cost",
                format!(
                    "l1_block_number={}, batch_id={}, tx_hash={:?}",
                    l1_block_number, verified.batch_id, l1_tx_hash
                ),
            )
            .await?;

            info!(
                l1_block_number,
                batch_id = verified.batch_id,
                tx_hash = ?l1_tx_hash,
                cost,
                "Inserted verify cost"
            );
        }

        Ok(())
    }

    /// Handle `L1Header` event with database insertion and preconf data processing
    async fn handle_l1_header_event(
        writer: &ClickhouseWriter,
        extractor: &Extractor,
        header: primitives::headers::L1Header,
    ) -> Result<()> {
        // Insert L1 header
        Self::with_db_error_context(
            writer.insert_l1_header(&header),
            "insert L1 header",
            format!("header_number={}", header.number),
        )
        .await?;

        info!(header_number = header.number, "Inserted L1 header");

        // Process preconfirmation data (same as original driver)
        Self::process_preconf_data(writer, extractor, &header).await;

        Ok(())
    }

    /// Shared database error handling utility to reduce code duplication
    async fn with_db_error_context<F, T>(future: F, operation: &str, context: String) -> Result<T>
    where
        F: std::future::Future<Output = Result<T, eyre::Error>>,
    {
        future.await.map_err(|e| {
            tracing::error!(
                err = %e,
                operation = operation,
                context = context,
                "Database operation failed"
            );
            eyre::eyre!("Failed to {}: {} - {}", operation, context, e)
        })
    }

    /// Fetch transaction cost for a given transaction hash
    async fn fetch_transaction_cost(extractor: &Extractor, tx_hash: B256) -> Option<u128> {
        if tx_hash.is_zero() {
            tracing::debug!("Skipping cost calculation for zero transaction hash");
            return None;
        }

        match extractor.get_receipt(tx_hash).await {
            Ok(receipt) => Some(primitives::l1_data_cost::cost_from_receipt(&receipt)),
            Err(e) => {
                tracing::error!(tx_hash = ?tx_hash, err = %e, "Failed to fetch receipt");
                None
            }
        }
    }

    /// Handle L2 header with duplicate filtering and reorg detection
    async fn handle_l2_header(
        writer: &ClickhouseWriter,
        clickhouse_reader: &Option<ClickhouseReader>,
        extractor: &Extractor,
        reorg_detector: &mut ReorgDetector,
        last_l2_header: &mut Option<(u64, Address)>,
        processed_l2_headers: &mut VecDeque<BlockHash>,
        header: primitives::headers::L2Header,
    ) -> Result<()> {
        // Check if this header has already been processed to avoid duplicate processing
        if processed_l2_headers.contains(&header.hash) {
            tracing::warn!(
                header_number = header.number,
                header_hash = ?header.hash,
                "Duplicate L2Header detected from RPC, skipping processing"
            );
            return Ok(());
        }

        // Add header to FIFO set, maintaining capacity of 1000
        processed_l2_headers.push_back(header.hash);
        if processed_l2_headers.len() > 1000 {
            processed_l2_headers.pop_front();
        }

        // Process reorg detection for new headers
        Self::process_reorg_detection(
            writer,
            clickhouse_reader,
            reorg_detector,
            last_l2_header,
            &header,
        )
        .await;

        // Insert L2 header with block statistics
        Self::insert_l2_header_with_stats(writer, extractor, &header).await;

        Ok(())
    }

    /// Process reorg detection and handle orphaned blocks/hashes
    async fn process_reorg_detection(
        writer: &ClickhouseWriter,
        clickhouse_reader: &Option<ClickhouseReader>,
        reorg_detector: &mut ReorgDetector,
        last_l2_header: &mut Option<(u64, Address)>,
        header: &primitives::headers::L2Header,
    ) {
        let prev_header = *last_l2_header;
        let old_head = reorg_detector.head_number();

        let reorg_result =
            reorg_detector.on_new_block_with_hash(header.number, B256::from(*header.hash));
        *last_l2_header = Some((header.number, header.beneficiary));

        if let Some((depth, orphaned_hash)) = reorg_result {
            Self::handle_reorg_event(
                writer,
                clickhouse_reader,
                prev_header,
                old_head,
                header,
                depth,
                orphaned_hash,
            )
            .await;
        }
    }

    /// Handle a detected reorg event by inserting reorg data and orphaned hashes
    async fn handle_reorg_event(
        writer: &ClickhouseWriter,
        clickhouse_reader: &Option<ClickhouseReader>,
        prev_header: Option<(u64, Address)>,
        old_head: u64,
        header: &primitives::headers::L2Header,
        depth: u16,
        orphaned_hash: Option<B256>,
    ) {
        let old_seq = prev_header.map(|(_, addr)| addr).unwrap_or(Address::ZERO);

        // Insert reorg event
        if let Err(e) =
            writer.insert_l2_reorg(header.number, depth, old_seq, header.beneficiary).await
        {
            tracing::error!(block_number = header.number, depth, err = %e, "Failed to insert L2 reorg");
        } else {
            info!(new_head = header.number, depth, "Inserted L2 reorg");
        }

        // Handle orphaned hash from one-block reorg
        if let Some(hash) = orphaned_hash {
            Self::insert_orphaned_hash(writer, hash, header.number).await;
        }

        // Handle orphaned blocks from traditional reorg
        if depth > 0 {
            Self::handle_traditional_reorg_orphans(
                writer,
                clickhouse_reader,
                old_head,
                header.number,
                depth,
            )
            .await;
        }
    }

    /// Insert a single orphaned hash
    async fn insert_orphaned_hash(writer: &ClickhouseWriter, hash: B256, block_number: u64) {
        if let Err(e) =
            writer.insert_orphaned_hashes(&[(HashBytes::from(hash), block_number)]).await
        {
            tracing::error!(block_number, orphaned_hash = ?hash, err = %e, "Failed to insert orphaned hash");
        } else {
            info!(block_number, orphaned_hash = ?hash, "Inserted orphaned hash");
        }
    }

    /// Handle orphaned blocks from traditional reorgs
    async fn handle_traditional_reorg_orphans(
        writer: &ClickhouseWriter,
        clickhouse_reader: &Option<ClickhouseReader>,
        old_head: u64,
        new_head: u64,
        depth: u16,
    ) {
        let orphaned_block_numbers = calculate_orphaned_blocks(old_head, new_head, depth.into());
        if orphaned_block_numbers.is_empty() {
            return;
        }

        let Some(reader) = clickhouse_reader else {
            return;
        };

        match reader.get_latest_hashes_for_blocks(&orphaned_block_numbers).await {
            Ok(orphaned_hashes) if !orphaned_hashes.is_empty() => {
                if let Err(e) = writer.insert_orphaned_hashes(&orphaned_hashes).await {
                    tracing::error!(count = orphaned_hashes.len(), err = %e, "Failed to insert orphaned hashes");
                } else {
                    info!(count = orphaned_hashes.len(), "Inserted orphaned hashes for reorg");
                }
            }
            Ok(_) => {} // No orphaned hashes found
            Err(e) => tracing::error!(err = %e, "Failed to fetch orphaned hashes"),
        }
    }

    /// Insert L2 header with calculated block statistics
    async fn insert_l2_header_with_stats(
        writer: &ClickhouseWriter,
        extractor: &Extractor,
        header: &primitives::headers::L2Header,
    ) {
        let (sum_gas_used, sum_tx, sum_priority_fee) = extractor
            .get_l2_block_stats(header.number, header.base_fee_per_gas)
            .await
            .unwrap_or_else(|e| {
                tracing::error!(header_number = header.number, err = %e, "Failed to get L2 block stats, using defaults");
                (0, 0, 0)
            });

        let sum_base_fee =
            sum_gas_used.saturating_mul(header.base_fee_per_gas.unwrap_or(0) as u128);

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
            tracing::error!(header_number = header.number, err = %e, "Failed to insert L2 header");
        } else {
            info!(
                header_number = header.number,
                sum_gas_used, sum_tx, "Inserted L2 header with stats"
            );
        }
    }

    /// Spawn all background monitors used by the processor.
    ///
    /// Each monitor runs in its own task and reports incidents via the
    /// [`IncidentClient`].
    fn spawn_monitors(&self) {
        if !self.instatus_monitors_enabled {
            return;
        }

        if let Some(url) = &self.public_rpc_url {
            tracing::info!(url = url.as_str(), "public rpc monitor enabled");
            let incident = self.instatus_monitors_enabled.then(|| {
                (self.incident_client.clone(), self.instatus_public_api_component_id.clone())
            });
            spawn_public_rpc_monitor(url.clone(), incident);
        }

        // Only spawn monitors if we have a clickhouse reader (database writes enabled)
        if let Some(reader) = &self.clickhouse_reader {
            InstatusL1Monitor::new(
                reader.clone(),
                self.incident_client.clone(),
                self.instatus_batch_submission_component_id.clone(),
                Duration::from_secs(self.instatus_l1_monitor_threshold_secs),
                Duration::from_secs(self.instatus_monitor_poll_interval_secs),
            )
            .spawn();

            InstatusMonitor::new(
                reader.clone(),
                self.incident_client.clone(),
                self.instatus_transaction_sequencing_component_id.clone(),
                Duration::from_secs(self.instatus_l2_monitor_threshold_secs),
                Duration::from_secs(self.instatus_monitor_poll_interval_secs),
            )
            .spawn();

            BatchProofTimeoutMonitor::new(
                reader.clone(),
                self.incident_client.clone(),
                self.instatus_proof_submission_component_id.clone(),
                Duration::from_secs(self.batch_proof_timeout_secs),
                Duration::from_secs(60),
            )
            .spawn();

            BatchVerifyTimeoutMonitor::new(
                reader.clone(),
                self.incident_client.clone(),
                self.instatus_proof_verification_component_id.clone(),
                Duration::from_secs(self.batch_proof_timeout_secs),
                Duration::from_secs(60),
            )
            .spawn();
        } else {
            tracing::warn!(
                "Instatus monitors enabled but no ClickHouse reader available (database writes disabled)"
            );
        }
    }

    /// Process preconfirmation data for L1 headers (ported from original driver)
    async fn process_preconf_data(
        writer: &ClickhouseWriter,
        extractor: &Extractor,
        header: &primitives::headers::L1Header,
    ) {
        // Get operator candidates for current epoch
        let opt_candidates = match extractor.get_operator_candidates_for_current_epoch().await {
            Ok(c) => {
                tracing::info!(
                    slot = header.slot,
                    block = header.number,
                    candidates = ?c,
                    candidates_count = c.len(),
                    "Successfully retrieved operator candidates"
                );
                Some(c)
            }
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

        let candidates = opt_candidates.unwrap_or_else(Vec::new);

        // Get current operator for epoch
        let opt_current_operator = match extractor.get_operator_for_current_epoch().await {
            Ok(op) => {
                info!(
                    block = header.number,
                    operator = ?op,
                    "Current operator for epoch"
                );
                Some(op)
            }
            Err(e) => {
                tracing::error!(block = header.number, err = %e, "get_operator_for_current_epoch failed");
                None
            }
        };

        // Get next operator for epoch
        let opt_next_operator = match extractor.get_operator_for_next_epoch().await {
            Ok(op) => {
                info!(
                    block = header.number,
                    operator = ?op,
                    "Next operator for epoch"
                );
                Some(op)
            }
            Err(e) => {
                // The first slot in the epoch doesn't have any next operator
                if header.slot % EPOCH_SLOTS != 0 {
                    tracing::error!(block = header.number, err = %e, "get_operator_for_next_epoch failed");
                }
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
                tracing::error!(slot = header.slot, err = %e, "Failed to insert preconf data");
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
}

/// Calculate average cost per batch for batch operations
const fn average_cost_per_batch(total_cost: u128, num_batches: usize) -> u128 {
    if num_batches == 0 { 0 } else { total_cost / num_batches as u128 }
}

/// Calculate orphaned block numbers during a reorg
///
/// # Arguments
/// * `old_head` - The head block number before the reorg
/// * `new_head` - The head block number after the reorg
/// * `depth` - The depth of the reorg
///
/// # Returns
/// Vector of block numbers that are orphaned (from `new_head+1` to `old_head` inclusive)
fn calculate_orphaned_blocks(old_head: u64, new_head: u64, _depth: u32) -> Vec<u64> {
    // Correct implementation: orphaned blocks are from new_head+1 to old_head (inclusive)
    if new_head >= old_head {
        // No orphaned blocks if new_head is >= old_head
        return Vec::new();
    }

    let orphaned_start = new_head + 1;
    let orphaned_end = old_head + 1; // +1 because range is exclusive at end
    (orphaned_start..orphaned_end).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Address, B256};
    use chainio::{ITaikoInbox, taiko::wrapper::ITaikoWrapper};
    use clickhouse::{
        AddressBytes, BatchBlockRow, BatchRow, ForcedInclusionProcessedRow, HashBytes,
        ProvedBatchRow, VerifiedBatchRow,
    };
    use clickhouse_rs::test::{Mock, handlers};
    use config::{
        ApiOpts, ClickhouseOpts, InstatusOpts, NatsOpts, NatsStreamOpts, Opts, RpcOpts,
        TaikoAddressOpts,
    };
    use futures::future;
    use messages::ForcedInclusionProcessedWrapper;
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_async;
    use url::Url;

    async fn start_ws_server() -> (Url, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                let _ = accept_async(stream).await;
                future::pending::<()>().await;
            }
        });
        let url = Url::parse(&format!("ws://{}", addr)).unwrap();
        (url, handle)
    }

    fn make_opts(url: Url, nats_url: Url, l1_url: Url, l2_url: Url) -> Opts {
        Opts {
            clickhouse: ClickhouseOpts {
                url,
                db: "test".into(),
                username: "user".into(),
                password: "pass".into(),
            },
            nats: NatsOpts { username: Some("natsuser".into()), password: Some("natspass".into()) },
            nats_stream: NatsStreamOpts {
                duplicate_window_secs: 120,
                storage_type: "file".into(),
                retention_policy: "workqueue".into(),
            },
            rpc: RpcOpts { l1_url, l2_url, public_url: None },
            api: ApiOpts {
                host: "127.0.0.1".into(),
                port: 3000,
                allowed_origins: Vec::new(),
                rate_limit_max_requests: 1000,
                rate_limit_period_secs: 60,
            },
            nats_url: nats_url.to_string(),
            taiko_addresses: TaikoAddressOpts {
                inbox_address: Address::ZERO,
                preconf_whitelist_address: Address::ZERO,
                taiko_wrapper_address: Address::ZERO,
            },
            instatus: InstatusOpts {
                api_key: "key".into(),
                page_id: "page".into(),
                batch_submission_component_id: "batch".into(),
                proof_submission_component_id: "proof".into(),
                proof_verification_component_id: "verify".into(),
                transaction_sequencing_component_id: "l2".into(),
                public_api_component_id: "public".into(),
                monitors_enabled: true,
                monitor_poll_interval_secs: 30,
                l1_monitor_threshold_secs: 96,
                l2_monitor_threshold_secs: 96,
                batch_proof_timeout_secs: 999,
            },
            enable_db_writes: false,
            reset_db: false,
            skip_migrations: false,
        }
    }

    #[test]
    fn average_cost_per_batch_even_split() {
        let cost = average_cost_per_batch(100, 4);
        assert_eq!(cost, 25);
    }

    #[test]
    fn average_cost_per_batch_rounds_down() {
        let cost = average_cost_per_batch(10, 3);
        assert_eq!(cost, 3);
    }

    #[test]
    fn calculate_orphaned_blocks_correct_behavior() {
        // Test case 1: old_head=10, new_head=8, depth=2
        // Expected: blocks 9,10 are orphaned
        let result = calculate_orphaned_blocks(10, 8, 2);
        assert_eq!(result, vec![9, 10], "Should return orphaned blocks [9,10]");

        // Test case 2: old_head=5, new_head=4, depth=1 (depth-1 reorg)
        // Expected: blocks [5] are orphaned
        let result2 = calculate_orphaned_blocks(5, 4, 1);
        assert_eq!(result2, vec![5], "Should return orphaned blocks [5]");

        // Test case 3: old_head=15, new_head=12, depth=3
        // Expected: blocks 13,14,15 are orphaned
        let result3 = calculate_orphaned_blocks(15, 12, 3);
        assert_eq!(result3, vec![13, 14, 15], "Should return orphaned blocks [13,14,15]");

        // Test case 4: No reorg (new_head >= old_head)
        let result4 = calculate_orphaned_blocks(10, 12, 0);
        let expected4: Vec<u64> = vec![];
        assert_eq!(
            result4, expected4,
            "Should return no orphaned blocks when new_head >= old_head"
        );

        // Test case 5: Adjacent blocks (old_head=5, new_head=4)
        let result5 = calculate_orphaned_blocks(5, 4, 1);
        assert_eq!(result5, vec![5], "Should return [5] when old_head=5, new_head=4");
    }

    #[test]
    fn fifo_set_behavior() {
        let mut fifo = VecDeque::new();

        // Test adding items
        let hash1 = B256::from([1u8; 32]);
        let hash2 = B256::from([2u8; 32]);
        let hash3 = B256::from([3u8; 32]);

        fifo.push_back(hash1);
        fifo.push_back(hash2);
        fifo.push_back(hash3);

        assert_eq!(fifo.len(), 3);
        assert!(fifo.contains(&hash1));
        assert!(fifo.contains(&hash2));
        assert!(fifo.contains(&hash3));

        // Test FIFO capacity management (simulate 1000 limit)
        const TEST_LIMIT: usize = 5;
        let mut limited_fifo = VecDeque::new();

        // Add more items than the limit
        for i in 1..=10 {
            let hash = B256::from([i as u8; 32]);
            limited_fifo.push_back(hash);

            if limited_fifo.len() > TEST_LIMIT {
                limited_fifo.pop_front();
            }
        }

        assert_eq!(limited_fifo.len(), TEST_LIMIT);
        // Should contain hashes 6-10, not 1-5
        assert!(!limited_fifo.contains(&B256::from([1u8; 32])));
        assert!(!limited_fifo.contains(&B256::from([5u8; 32])));
        assert!(limited_fifo.contains(&B256::from([6u8; 32])));
        assert!(limited_fifo.contains(&B256::from([10u8; 32])));
    }

    #[tokio::test]
    async fn handle_batch_proposed_event_inserts_row() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<BatchRow>());
        let _ctl_blocks = mock.add(handlers::record::<BatchBlockRow>());

        let clickhouse = ClickhouseWriter::new(
            mock.url().parse().unwrap(),
            "test".into(),
            "user".into(),
            "pass".into(),
        );

        let batch = ITaikoInbox::BatchProposed {
            info: ITaikoInbox::BatchInfo {
                proposedIn: 2,
                blobByteSize: 50,
                blocks: vec![ITaikoInbox::BlockParams::default(); 1],
                blobHashes: vec![B256::repeat_byte(1)],
                lastBlockId: 100,
                ..Default::default()
            },
            meta: ITaikoInbox::BatchMetadata {
                proposer: Address::repeat_byte(2),
                batchId: 7,
                ..Default::default()
            },
            ..Default::default()
        };

        // Test the batch insertion directly since cost calculation is skipped with zero tx_hash
        clickhouse.insert_batch(&batch, B256::ZERO).await.unwrap();

        let rows: Vec<BatchRow> = ctl.collect().await;
        assert_eq!(
            rows,
            vec![BatchRow {
                l1_block_number: 2,
                l1_tx_hash: HashBytes::from([0u8; 32]),
                batch_id: 7,
                batch_size: 1,
                last_l2_block_number: 100,
                proposer_addr: AddressBytes::from(Address::repeat_byte(2)),
                blob_count: 1,
                blob_total_bytes: 50,
            }]
        );
    }

    #[tokio::test]
    async fn handle_forced_inclusion_event_inserts_row() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<ForcedInclusionProcessedRow>());

        let url = Url::parse(mock.url()).unwrap();
        let (l1_url, l1_handle) = start_ws_server().await;
        let (l2_url, l2_handle) = start_ws_server().await;
        let nats_url = Url::parse("nats://localhost:4222").unwrap();

        let opts = make_opts(url, nats_url, l1_url.clone(), l2_url.clone());
        let clickhouse = ClickhouseWriter::new(
            opts.clickhouse.url,
            opts.clickhouse.db,
            opts.clickhouse.username,
            opts.clickhouse.password,
        );

        l1_handle.abort();
        l2_handle.abort();

        let event = ITaikoWrapper::ForcedInclusionProcessed {
            forcedInclusion: ITaikoWrapper::ForcedInclusion {
                blobHash: B256::repeat_byte(5),
                feeInGwei: 1,
                createdAtBatchId: 0,
                blobByteOffset: 0,
                blobByteSize: 0,
                blobCreatedIn: 0,
            },
        };

        let wrapper = ForcedInclusionProcessedWrapper::from(event);
        ProcessorDriver::handle_forced_inclusion_event(&clickhouse, wrapper).await.unwrap();

        let rows: Vec<ForcedInclusionProcessedRow> = ctl.collect().await;
        assert_eq!(
            rows,
            vec![ForcedInclusionProcessedRow { blob_hash: HashBytes::from([5u8; 32]) }]
        );
    }

    #[tokio::test]
    async fn handle_batches_proved_event_inserts_rows() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<ProvedBatchRow>());

        let clickhouse = ClickhouseWriter::new(
            mock.url().parse().unwrap(),
            "test".into(),
            "user".into(),
            "pass".into(),
        );

        let transition = ITaikoInbox::Transition {
            parentHash: B256::repeat_byte(1),
            blockHash: B256::repeat_byte(2),
            stateRoot: B256::repeat_byte(3),
        };
        let proved = ITaikoInbox::BatchesProved {
            verifier: Address::repeat_byte(4),
            batchIds: vec![8],
            transitions: vec![transition],
        };

        // Test the proved batch insertion directly
        clickhouse.insert_proved_batch(&proved, 10).await.unwrap();

        let rows: Vec<ProvedBatchRow> = ctl.collect().await;
        assert_eq!(
            rows,
            vec![ProvedBatchRow {
                l1_block_number: 10,
                batch_id: 8,
                verifier_addr: AddressBytes::from(Address::repeat_byte(4)),
                parent_hash: HashBytes::from([1u8; 32]),
                block_hash: HashBytes::from([2u8; 32]),
                state_root: HashBytes::from([3u8; 32]),
            }]
        );
    }

    #[tokio::test]
    async fn handle_batches_verified_event_inserts_row() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<VerifiedBatchRow>());

        let clickhouse = ClickhouseWriter::new(
            mock.url().parse().unwrap(),
            "test".into(),
            "user".into(),
            "pass".into(),
        );

        let verified = chainio::BatchesVerified { batch_id: 3, block_hash: [9u8; 32] };

        // Test the verified batch insertion directly
        clickhouse.insert_verified_batch(&verified, 12).await.unwrap();

        let rows: Vec<VerifiedBatchRow> = ctl.collect().await;
        assert_eq!(
            rows,
            vec![VerifiedBatchRow {
                l1_block_number: 12,
                batch_id: 3,
                block_hash: HashBytes::from([9u8; 32]),
            }]
        );
    }
}
