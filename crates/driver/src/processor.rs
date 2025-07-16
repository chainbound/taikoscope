//! Taikoscope Processor Driver

use std::time::Duration;

use alloy_primitives::{Address, B256};
use clickhouse::{AddressBytes, ClickhouseReader, ClickhouseWriter, HashBytes, L2HeadEvent};
use config::Opts;
use extractor::{Extractor, ReorgDetector};
use eyre::Result;
use incident::{
    BatchProofTimeoutMonitor, InstatusL1Monitor, InstatusMonitor, Monitor,
    client::Client as IncidentClient, monitor::BatchVerifyTimeoutMonitor,
};
use nats_utils::TaikoEvent;
use network::public_rpc_monitor;
use tokio_stream::StreamExt;
use tracing::info;
use url::Url;

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
    instatus_monitors_enabled: bool,
    instatus_monitor_poll_interval_secs: u64,
    instatus_monitor_threshold_secs: u64,
    batch_proof_timeout_secs: u64,
    public_rpc_url: Option<Url>,
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

        let nats_client = async_nats::connect(&opts.nats_url).await?;
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

        info!("🚀 Running database migrations...");
        migration_writer.init_db(opts.reset_db).await?;
        info!("✅ Database migrations completed");

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
            incident_client,
        ) = if opts.instatus.monitors_enabled {
            (
                opts.instatus.batch_submission_component_id.clone(),
                opts.instatus.proof_submission_component_id.clone(),
                opts.instatus.proof_verification_component_id.clone(),
                opts.instatus.transaction_sequencing_component_id.clone(),
                IncidentClient::new(opts.instatus.api_key.clone(), opts.instatus.page_id.clone()),
            )
        } else {
            (
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
            instatus_monitors_enabled: opts.instatus.monitors_enabled,
            instatus_monitor_poll_interval_secs: opts.instatus.monitor_poll_interval_secs,
            instatus_monitor_threshold_secs: opts.instatus.monitor_threshold_secs,
            batch_proof_timeout_secs: opts.instatus.batch_proof_timeout_secs,
            public_rpc_url: opts.rpc.public_url,
        })
    }

    /// Start the processor event loop, consuming NATS events and processing them
    pub async fn start(self) -> Result<()> {
        info!("Starting processor event loop");

        if self.enable_db_writes {
            info!("Database writes ENABLED - events will be processed and stored");
        } else {
            info!("Database writes DISABLED - events will be logged and dropped");
        }

        // Spawn monitors before starting the event loop
        self.spawn_monitors();

        let nats_client = self.nats_client;
        let clickhouse_writer = self.clickhouse_writer;
        let clickhouse_reader = self.clickhouse_reader;
        let extractor = self.extractor;
        let mut reorg_detector = self.reorg_detector;
        let mut last_l2_header = self.last_l2_header;
        let enable_db_writes = self.enable_db_writes;

        let jetstream = async_nats::jetstream::new(nats_client);

        // Get or create the stream first
        let _stream = jetstream
            .get_or_create_stream(async_nats::jetstream::stream::Config {
                name: "taiko".to_owned(),
                subjects: vec!["taiko.events".to_owned()],
                ..Default::default()
            })
            .await?;

        // Create the consumer
        let consumer = jetstream
            .create_consumer_on_stream(
                async_nats::jetstream::consumer::pull::Config {
                    durable_name: Some("processor".to_owned()),
                    ..Default::default()
                },
                "taiko",
            )
            .await?;

        let mut messages = consumer.messages().await?;

        while let Some(message) = messages.next().await {
            match message {
                Ok(msg) => {
                    // Try to process the message with retry logic
                    let mut retries = 0;
                    const MAX_RETRIES: u32 = 3;

                    loop {
                        match Self::process_message(
                            &clickhouse_writer,
                            &clickhouse_reader,
                            &extractor,
                            &mut reorg_detector,
                            &mut last_l2_header,
                            enable_db_writes,
                            &msg,
                        )
                        .await
                        {
                            Ok(()) => {
                                // Success - acknowledge the message
                                if let Err(e) = msg.ack().await {
                                    tracing::error!(err = %e, "Failed to ack message");
                                }
                                break;
                            }
                            Err(e) => {
                                if retries >= MAX_RETRIES {
                                    tracing::error!(
                                        err = %e,
                                        retries = retries,
                                        "Failed to process message after all retries, nacking message"
                                    );
                                    // Nack the message to put it back in the queue
                                    if let Err(nack_err) = msg
                                        .ack_with(async_nats::jetstream::AckKind::Nak(None))
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
                                tokio::time::sleep(Duration::from_millis(100 * (1 << retries)))
                                    .await;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(err = %e, "Error receiving message from NATS");
                    // Wait a bit before continuing to avoid tight loop on persistent errors
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                }
            }
        }

        Ok(())
    }

    async fn process_message(
        clickhouse_writer: &Option<ClickhouseWriter>,
        clickhouse_reader: &Option<ClickhouseReader>,
        extractor: &Extractor,
        reorg_detector: &mut ReorgDetector,
        last_l2_header: &mut Option<(u64, Address)>,
        enable_db_writes: bool,
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
                    event,
                )
                .await
            } else {
                tracing::error!("Database writes enabled but no writer available");
                Ok(())
            }
        } else {
            Self::process_event_log_and_drop(event).await
        }
    }

    async fn process_event_with_db_write(
        writer: &ClickhouseWriter,
        clickhouse_reader: &Option<ClickhouseReader>,
        extractor: &Extractor,
        reorg_detector: &mut ReorgDetector,
        last_l2_header: &mut Option<(u64, Address)>,
        event: TaikoEvent,
    ) -> Result<()> {
        match event {
            TaikoEvent::BatchProposed(wrapper) => {
                let batch = &wrapper.batch;
                let l1_tx_hash = wrapper.l1_tx_hash;

                if let Err(e) = writer.insert_batch(batch, l1_tx_hash).await {
                    tracing::error!(batch_last_block = ?batch.last_block_number(), err = %e, "Failed to insert batch");
                } else {
                    info!(last_block_number = ?batch.last_block_number(), "Inserted batch");
                }

                // Calculate and insert L1 data cost
                if let Some(cost) = Self::fetch_transaction_cost(extractor, l1_tx_hash).await {
                    if let Err(e) = writer
                        .insert_l1_data_cost(batch.info.proposedIn, batch.meta.batchId, cost)
                        .await
                    {
                        tracing::error!(
                            l1_block_number = batch.info.proposedIn,
                            tx_hash = ?l1_tx_hash,
                            err = %e,
                            "Failed to insert L1 data cost"
                        );
                    } else {
                        info!(
                            l1_block_number = batch.info.proposedIn,
                            tx_hash = ?l1_tx_hash,
                            cost,
                            "Inserted L1 data cost"
                        );
                    }
                }
            }
            TaikoEvent::ForcedInclusionProcessed(wrapper) => {
                let event = &wrapper.event;
                if let Err(e) = writer.insert_forced_inclusion(event).await {
                    tracing::error!(blob_hash = ?event.blobHash, err = %e, "Failed to insert forced inclusion");
                } else {
                    info!(blob_hash = ?event.blobHash, "Inserted forced inclusion");
                }
            }
            TaikoEvent::BatchesProved(wrapper) => {
                let proved = &wrapper.proved;
                let l1_block_number = wrapper.l1_block_number;
                let l1_tx_hash = wrapper.l1_tx_hash;

                if let Err(e) = writer.insert_proved_batch(proved, l1_block_number).await {
                    tracing::error!(batch_ids = ?proved.batch_ids_proved(), err = %e, "Failed to insert proved batch");
                } else {
                    info!(batch_ids = ?proved.batch_ids_proved(), "Inserted proved batch");
                }

                // Calculate and insert prove costs
                if let Some(cost) = Self::fetch_transaction_cost(extractor, l1_tx_hash).await {
                    let cost_per_batch =
                        Self::average_cost_per_batch(cost, proved.batch_ids_proved().len());
                    for batch_id in proved.batch_ids_proved() {
                        if let Err(e) = writer
                            .insert_prove_cost(l1_block_number, *batch_id, cost_per_batch)
                            .await
                        {
                            tracing::error!(
                                l1_block_number,
                                batch_id,
                                tx_hash = ?l1_tx_hash,
                                err = %e,
                                "Failed to insert prove cost"
                            );
                        } else {
                            info!(
                                l1_block_number,
                                batch_id,
                                tx_hash = ?l1_tx_hash,
                                cost = cost_per_batch,
                                "Inserted prove cost"
                            );
                        }
                    }
                }
            }
            TaikoEvent::BatchesVerified(wrapper) => {
                let verified = &wrapper.verified;
                let l1_block_number = wrapper.l1_block_number;
                let l1_tx_hash = wrapper.l1_tx_hash;

                if let Err(e) = writer.insert_verified_batch(verified, l1_block_number).await {
                    tracing::error!(batch_id = verified.batch_id, err = %e, "Failed to insert verified batch");
                } else {
                    info!(batch_id = verified.batch_id, "Inserted verified batch");
                }

                // Calculate and insert verify cost
                if let Some(cost) = Self::fetch_transaction_cost(extractor, l1_tx_hash).await {
                    if let Err(e) =
                        writer.insert_verify_cost(l1_block_number, verified.batch_id, cost).await
                    {
                        tracing::error!(
                            l1_block_number,
                            batch_id = verified.batch_id,
                            tx_hash = ?l1_tx_hash,
                            err = %e,
                            "Failed to insert verify cost"
                        );
                    } else {
                        info!(
                            l1_block_number,
                            batch_id = verified.batch_id,
                            tx_hash = ?l1_tx_hash,
                            cost,
                            "Inserted verify cost"
                        );
                    }
                }
            }
            TaikoEvent::L1Header(header) => {
                if let Err(e) = writer.insert_l1_header(&header).await {
                    tracing::error!(header_number = header.number, err = %e, "Failed to insert L1 header");
                } else {
                    info!(header_number = header.number, "Inserted L1 header");
                }

                // Process preconfirmation data like the original driver
                Self::process_preconf_data(writer, extractor, &header).await;
            }
            TaikoEvent::L2Header(header) => {
                Self::handle_l2_header(
                    writer,
                    clickhouse_reader,
                    extractor,
                    reorg_detector,
                    last_l2_header,
                    header,
                )
                .await?;
            }
        }
        Ok(())
    }

    async fn process_event_log_and_drop(event: TaikoEvent) -> Result<()> {
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
                    blob_hash = ?wrapper.event.blobHash,
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
                info!(
                    header_number = header.number,
                    "Received L2Header event (dropped - DB writes disabled)"
                );
            }
        }
        Ok(())
    }

    /// Fetch transaction cost for a given transaction hash
    async fn fetch_transaction_cost(extractor: &Extractor, tx_hash: B256) -> Option<u128> {
        match extractor.get_receipt(tx_hash).await {
            Ok(receipt) => Some(primitives::l1_data_cost::cost_from_receipt(&receipt)),
            Err(e) => {
                tracing::error!(tx_hash = ?tx_hash, err = %e, "Failed to fetch receipt");
                None
            }
        }
    }

    /// Calculate average cost per batch for batch operations
    const fn average_cost_per_batch(total_cost: u128, num_batches: usize) -> u128 {
        if num_batches == 0 { 0 } else { total_cost / num_batches as u128 }
    }

    /// Handle L2 header with reorg detection
    async fn handle_l2_header(
        writer: &ClickhouseWriter,
        clickhouse_reader: &Option<ClickhouseReader>,
        extractor: &Extractor,
        reorg_detector: &mut ReorgDetector,
        last_l2_header: &mut Option<(u64, Address)>,
        header: primitives::headers::L2Header,
    ) -> Result<()> {
        let prev_header = *last_l2_header;
        let old_head = reorg_detector.head_number(); // Capture old head before detection

        // Detect reorgs
        let reorg_depth = reorg_detector.on_new_block(header.number);
        *last_l2_header = Some((header.number, header.beneficiary));

        if let Some(depth) = reorg_depth {
            let old_seq = prev_header.map(|(_, addr)| addr).unwrap_or(Address::ZERO);

            // Insert reorg event
            if let Err(e) =
                writer.insert_l2_reorg(header.number, depth, old_seq, header.beneficiary).await
            {
                tracing::error!(block_number = header.number, depth = depth, err = %e, "Failed to insert L2 reorg");
            } else {
                info!(new_head = header.number, depth, "Inserted L2 reorg");
            }

            // Handle orphaned blocks
            if depth > 0 {
                let orphaned_block_numbers =
                    calculate_orphaned_blocks(old_head, header.number, depth.into());

                if !orphaned_block_numbers.is_empty() {
                    if let Some(reader) = clickhouse_reader {
                        match reader.get_latest_hashes_for_blocks(&orphaned_block_numbers).await {
                            Ok(orphaned_hashes) => {
                                if !orphaned_hashes.is_empty() {
                                    if let Err(e) =
                                        writer.insert_orphaned_hashes(&orphaned_hashes).await
                                    {
                                        tracing::error!(
                                            count = orphaned_hashes.len(),
                                            err = %e,
                                            "Failed to insert orphaned hashes"
                                        );
                                    } else {
                                        info!(
                                            count = orphaned_hashes.len(),
                                            "Inserted orphaned hashes for reorg"
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!(err = %e, "Failed to fetch orphaned hashes");
                            }
                        }
                    }
                }
            }
        }

        // Calculate L2 block statistics using the extractor
        let (sum_gas_used, sum_tx, sum_priority_fee) = match extractor
            .get_l2_block_stats(header.number, header.base_fee_per_gas)
            .await
        {
            Ok(stats) => stats,
            Err(e) => {
                tracing::error!(header_number = header.number, err = %e, "Failed to get L2 block stats, using defaults");
                (0, 0, 0)
            }
        };

        // Calculate sum_base_fee using the base fee per gas and gas used
        let sum_base_fee =
            sum_gas_used.saturating_mul(header.base_fee_per_gas.unwrap_or(0) as u128);

        // Convert L2Header to L2HeadEvent format expected by ClickHouse
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

        Ok(())
    }

    /// Spawn all background monitors used by the processor.
    ///
    /// Each monitor runs in its own task and reports incidents via the
    /// [`IncidentClient`].
    fn spawn_monitors(&self) {
        if let Some(url) = &self.public_rpc_url {
            tracing::info!(url = url.as_str(), "public rpc monitor enabled");
            public_rpc_monitor::spawn_public_rpc_monitor(url.clone());
        }

        if !self.instatus_monitors_enabled {
            return;
        }

        // Only spawn monitors if we have a clickhouse reader (database writes enabled)
        if let Some(reader) = &self.clickhouse_reader {
            InstatusL1Monitor::new(
                reader.clone(),
                self.incident_client.clone(),
                self.instatus_batch_submission_component_id.clone(),
                Duration::from_secs(self.instatus_monitor_threshold_secs),
                Duration::from_secs(self.instatus_monitor_poll_interval_secs),
            )
            .spawn();

            InstatusMonitor::new(
                reader.clone(),
                self.incident_client.clone(),
                self.instatus_transaction_sequencing_component_id.clone(),
                Duration::from_secs(self.instatus_monitor_threshold_secs),
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
