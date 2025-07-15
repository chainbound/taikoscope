//! Taikoscope Processor Driver

use clickhouse::{AddressBytes, ClickhouseWriter, HashBytes, L2HeadEvent};
use config::Opts;
use eyre::Result;
use lru::LruCache;
use nats_utils::TaikoEvent;
use std::num::NonZeroUsize;
use tokio_stream::StreamExt;
use tracing::info;

/// Driver for the processor service that consumes NATS events and writes to `ClickHouse`
#[derive(Debug)]
pub struct ProcessorDriver {
    nats_client: async_nats::Client,
    clickhouse_writer: Option<ClickhouseWriter>,
    enable_db_writes: bool,
}

impl ProcessorDriver {
    /// Create a new processor driver with the given configuration
    pub async fn new(opts: Opts) -> Result<Self> {
        info!("Initializing processor driver");

        let nats_client = async_nats::connect(&opts.nats_url).await?;
        info!("Connected to NATS server at {}", opts.nats_url);

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

        Ok(Self { nats_client, clickhouse_writer, enable_db_writes: opts.enable_db_writes })
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
        let mut dedup_cache = LruCache::new(NonZeroUsize::new(10_000).unwrap());

        while let Some(message) = messages.next().await {
            match message {
                Ok(msg) => {
                    if let Err(e) = Self::process_message(
                        &clickhouse_writer,
                        enable_db_writes,
                        &mut dedup_cache,
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
        enable_db_writes: bool,
        dedup_cache: &mut LruCache<String, ()>,
        msg: &async_nats::jetstream::Message,
    ) -> Result<()> {
        let event: TaikoEvent = serde_json::from_slice(&msg.payload)?;
        let dedup_id = event.dedup_id();
        if dedup_cache.contains(&dedup_id) {
            tracing::debug!(%dedup_id, "Duplicate message received - skipping");
            return Ok(());
        }
        dedup_cache.put(dedup_id.clone(), ());

        if enable_db_writes {
            if let Some(writer) = clickhouse_writer {
                Self::process_event_with_db_write(writer, event).await
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

                if let Err(e) = writer.insert_proved_batch(proved, l1_block_number).await {
                    tracing::error!(batch_ids = ?proved.batch_ids_proved(), err = %e, "Failed to insert proved batch");
                } else {
                    info!(batch_ids = ?proved.batch_ids_proved(), "Inserted proved batch");
                }
            }
            TaikoEvent::BatchesVerified(wrapper) => {
                let verified = &wrapper.verified;
                let l1_block_number = wrapper.l1_block_number;

                if let Err(e) = writer.insert_verified_batch(verified, l1_block_number).await {
                    tracing::error!(batch_id = verified.batch_id, err = %e, "Failed to insert verified batch");
                } else {
                    info!(batch_id = verified.batch_id, "Inserted verified batch");
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
                // Convert L2Header to L2HeadEvent format expected by ClickHouse
                let event = L2HeadEvent {
                    l2_block_number: header.number,
                    block_hash: HashBytes(*header.hash),
                    block_ts: header.timestamp,
                    sum_gas_used: 0, // These would need to be calculated from block data
                    sum_tx: 0,
                    sum_priority_fee: 0,
                    sum_base_fee: 0,
                    sequencer: AddressBytes(header.beneficiary.into_array()),
                };

                if let Err(e) = writer.insert_l2_header(&event).await {
                    tracing::error!(header_number = header.number, err = %e, "Failed to insert L2 header");
                } else {
                    info!(header_number = header.number, "Inserted L2 header");
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
}
