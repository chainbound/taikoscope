#![allow(missing_docs)]
#![allow(unused_imports)]

use alloy_primitives::{Address, B256};
use clap::Parser;
use clickhouse::ClickhouseWriter;
use config::Opts;
use dotenvy::dotenv;
use messages::{BatchProposedWrapper, BatchesProvedWrapper, ForcedInclusionProcessedWrapper};
use nats_utils::TaikoEvent;
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

    info!("Processor starting...");

    // Connect to NATS
    let nats_client = async_nats::connect(&opts.nats_url).await?;
    let js = async_nats::jetstream::new(nats_client.clone());

    // Initialize ClickHouse client only if database writes are enabled
    let clickhouse_writer = if opts.enable_db_writes {
        info!("Database writes enabled - initializing ClickHouse client");
        let writer = ClickhouseWriter::new(
            opts.clickhouse.url.clone(),
            opts.clickhouse.db.clone(),
            opts.clickhouse.username.clone(),
            opts.clickhouse.password.clone(),
        );
        Some(writer)
    } else {
        info!("Database writes disabled - running in log-only mode");
        None
    };

    // Subscribe to events from NATS JetStream
    let stream = js
        .get_or_create_stream(async_nats::jetstream::stream::Config {
            name: "taiko_events".to_owned(),
            subjects: vec!["taiko.events".to_owned()],
            ..Default::default()
        })
        .await?;

    let consumer = stream
        .get_or_create_consumer(
            "processor",
            async_nats::jetstream::consumer::pull::Config {
                durable_name: Some("processor".to_owned()),
                ..Default::default()
            },
        )
        .await?;

    info!("Processor listening for events...");

    // Process events from NATS
    let mut messages = consumer.messages().await?;

    while let Some(message_result) = messages.next().await {
        match message_result {
            Ok(message) => {
                let payload = message.payload.clone();

                match serde_json::from_slice::<TaikoEvent>(&payload) {
                    Ok(event) => {
                        info!("Processing event: {}", event.dedup_id());

                        if let Some(ref writer) = clickhouse_writer {
                            if let Err(e) = process_event_with_db_write(writer, event).await {
                                tracing::error!(err = %e, "Failed to process event with database write");
                            }
                        } else {
                            // Log-only mode
                            info!("Event received (log-only): {}", event.dedup_id());
                        }

                        // Acknowledge the message
                        if let Err(e) = message.ack().await {
                            tracing::error!(err = %e, "Failed to acknowledge NATS message");
                        }
                    }
                    Err(e) => {
                        tracing::error!(err = %e, "Failed to deserialize event from NATS");
                        // Still acknowledge the message to avoid reprocessing
                        if let Err(ack_err) = message.ack().await {
                            tracing::error!(err = %ack_err, "Failed to acknowledge malformed NATS message");
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!(err = %e, "Failed to receive message from NATS consumer");
            }
        }
    }

    Ok(())
}

async fn process_event_with_db_write(
    writer: &ClickhouseWriter,
    event: TaikoEvent,
) -> eyre::Result<()> {
    match event {
        TaikoEvent::L1Header(header) => {
            if let Err(e) = writer.insert_l1_header(&header).await {
                tracing::error!(header_number = header.number, err = %e, "Failed to insert L1 header");
            } else {
                info!(header_number = header.number, "Inserted L1 header");
            }
        }
        TaikoEvent::L2Header(header) => {
            // Convert L2Header to L2HeadEvent format expected by ClickHouse
            let event = clickhouse::L2HeadEvent {
                l2_block_number: header.number,
                block_hash: clickhouse::HashBytes(*header.hash),
                block_ts: header.timestamp,
                sum_gas_used: 0, // These would need to be calculated from block data
                sum_tx: 0,
                sum_priority_fee: 0,
                sum_base_fee: 0,
                sequencer: clickhouse::AddressBytes(header.beneficiary.into_array()),
            };

            if let Err(e) = writer.insert_l2_header(&event).await {
                tracing::error!(block_number = header.number, err = %e, "Failed to insert L2 header");
            } else {
                info!(l2_header = header.number, block_ts = header.timestamp, "Inserted L2 header");
            }
        }
        TaikoEvent::BatchProposed(wrapper) => {
            let batch = wrapper.0;
            let l1_tx_hash = B256::ZERO; // In the ingestor, this was lost - would need to preserve it

            if let Err(e) = writer.insert_batch(&batch, l1_tx_hash).await {
                tracing::error!(batch_last_block = ?batch.last_block_number(), err = %e, "Failed to insert batch");
            } else {
                info!(last_block_number = ?batch.last_block_number(), "Inserted batch");
            }
        }
        TaikoEvent::ForcedInclusionProcessed(wrapper) => {
            let event = wrapper.0;
            if let Err(e) = writer.insert_forced_inclusion(&event).await {
                tracing::error!(blob_hash = ?event.blobHash, err = %e, "Failed to insert forced inclusion");
            } else {
                info!(blob_hash = ?event.blobHash, "Inserted forced inclusion processed");
            }
        }
        TaikoEvent::BatchesProved(wrapper) => {
            let proved = wrapper.0;
            let l1_block_number = 0; // This information was lost in the NATS event

            if let Err(e) = writer.insert_proved_batch(&proved, l1_block_number).await {
                tracing::error!(
                    batch_ids = ?proved.batch_ids_proved(),
                    err = %e,
                    "Failed to insert proved batch"
                );
            } else {
                info!(batch_ids = ?proved.batch_ids_proved(), "Inserted proved batch");
            }
        }
        TaikoEvent::BatchesVerified(verified) => {
            let l1_block_number = 0; // This information was lost in the NATS event

            if let Err(e) = writer.insert_verified_batch(&verified, l1_block_number).await {
                tracing::error!(batch_id = ?verified.batch_id, err = %e, "Failed to insert verified batch");
            } else {
                info!(batch_id = ?verified.batch_id, "Inserted verified batch");
            }
        }
    }

    Ok(())
}
