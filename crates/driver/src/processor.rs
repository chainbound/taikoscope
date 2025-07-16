//! Taikoscope Processor Driver

use alloy_primitives::B256;
use clickhouse::{AddressBytes, ClickhouseWriter, HashBytes, L2HeadEvent};
use config::Opts;
use extractor::Extractor;
use eyre::Result;
use nats_utils::TaikoEvent;
use tokio_stream::StreamExt;
use tracing::info;

/// Driver for the processor service that consumes NATS events and writes to `ClickHouse`
#[derive(Debug)]
pub struct ProcessorDriver {
    nats_client: async_nats::Client,
    clickhouse_writer: Option<ClickhouseWriter>,
    extractor: Extractor,
    enable_db_writes: bool,
}

impl ProcessorDriver {
    /// Create a new processor driver with the given configuration
    pub async fn new(opts: Opts) -> Result<Self> {
        info!("Initializing processor driver");

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
                opts.clickhouse.url,
                opts.clickhouse.db,
                opts.clickhouse.username,
                opts.clickhouse.password,
            )
        });

        Ok(Self {
            nats_client,
            clickhouse_writer,
            extractor,
            enable_db_writes: opts.enable_db_writes,
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

        let nats_client = self.nats_client;
        let clickhouse_writer = self.clickhouse_writer;
        let extractor = self.extractor;
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
                    if let Err(e) = Self::process_message(
                        &clickhouse_writer,
                        &extractor,
                        enable_db_writes,
                        &msg,
                    )
                    .await
                    {
                        tracing::error!(err = %e, "Failed to process message");
                    }
                    if let Err(e) = msg.ack().await {
                        tracing::error!(err = %e, "Failed to ack message");
                    }
                }
                Err(e) => {
                    tracing::error!(err = %e, "Error receiving message");
                }
            }
        }

        Ok(())
    }

    async fn process_message(
        clickhouse_writer: &Option<ClickhouseWriter>,
        extractor: &Extractor,
        enable_db_writes: bool,
        msg: &async_nats::jetstream::Message,
    ) -> Result<()> {
        let event: TaikoEvent = serde_json::from_slice(&msg.payload)?;

        if enable_db_writes {
            if let Some(writer) = clickhouse_writer {
                Self::process_event_with_db_write(writer, extractor, event).await
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
        extractor: &Extractor,
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
            }
            TaikoEvent::L2Header(header) => {
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

                // Calculate sum_base_fee using the base fee per gas and transaction count
                let sum_base_fee =
                    header.base_fee_per_gas.map(|base_fee| base_fee * sum_tx as u64).unwrap_or(0)
                        as u128;

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
}
