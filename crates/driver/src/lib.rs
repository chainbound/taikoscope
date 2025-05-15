//! Taikoscope Driver

use std::time::Duration;

use eyre::Result;
use tokio_stream::StreamExt;
use tracing::info;

use alloy_primitives::Address;
use chainio::{
    ITaikoInbox::BatchProposed, taiko::wrapper::ITaikoWrapper::ForcedInclusionProcessed,
};
use clickhouse::ClickhouseClient;
use config::Opts;
use extractor::{
    BatchProposedStream, Extractor, ForcedInclusionStream, L1Header, L1HeaderStream, L2Header,
    L2HeaderStream, ReorgDetector,
};
use incident::{InstatusMonitor, client::Client as IncidentClient};

/// An EPOCH is a series of 32 slots.
pub const EPOCH_SLOTS: u64 = 32;

/// Taikoscope Driver
#[derive(Debug)]
pub struct Driver {
    clickhouse: ClickhouseClient,
    extractor: Extractor,
    reorg: ReorgDetector,
    incident_client: IncidentClient,
    instatus_component_id: String,
}

impl Driver {
    /// Build everything (client, extractor, detector), but don't start the event loop yet.
    pub async fn new(opts: Opts) -> Result<Self> {
        // init db client
        let clickhouse = ClickhouseClient::new(
            opts.clickhouse_url.clone(),
            opts.clickhouse_db.clone(),
            opts.clickhouse_username.clone(),
            opts.clickhouse_password.clone(),
        )?;
        clickhouse.init_db(opts.reset_db).await?;

        // init extractor
        let extractor = Extractor::new(
            opts.l1_rpc_url.clone(),
            opts.l2_rpc_url.clone(),
            opts.inbox_address,
            opts.preconf_whitelist_address,
            opts.taiko_wrapper_address,
        )
        .await?;

        // init incident client and component ID
        let instatus_component_id = opts.instatus_component_id.clone();
        let incident_client =
            IncidentClient::new(opts.instatus_api_key.clone(), opts.instatus_page_id.clone());

        Ok(Self {
            clickhouse,
            extractor,
            reorg: ReorgDetector::new(),
            incident_client,
            instatus_component_id,
        })
    }

    async fn subscribe_l1(&self) -> L1HeaderStream {
        loop {
            match self.extractor.get_l1_header_stream().await {
                Ok(s) => return s,
                Err(e) => {
                    tracing::error!("L1 subscribe failed: {}. Retrying in 5s…", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn subscribe_l2(&self) -> L2HeaderStream {
        loop {
            match self.extractor.get_l2_header_stream().await {
                Ok(s) => return s,
                Err(e) => {
                    tracing::error!("L2 subscribe failed: {}. Retrying in 5s…", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn subscribe_batch(&self) -> BatchProposedStream {
        loop {
            match self.extractor.get_batch_proposed_stream().await {
                Ok(s) => return s,
                Err(e) => {
                    tracing::error!("BatchProposed subscribe failed: {}. Retrying in 5s…", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn subscribe_forced(&self) -> ForcedInclusionStream {
        loop {
            match self.extractor.get_forced_inclusion_stream().await {
                Ok(s) => return s,
                Err(e) => {
                    tracing::error!("ForcedInclusion subscribe failed: {}. Retrying in 5s…", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    /// Consume the driver and drive the infinite processing loop.
    pub async fn start(mut self) -> Result<()> {
        info!("Starting event loop...");

        let mut l1_stream = self.subscribe_l1().await;
        let mut l2_stream = self.subscribe_l2().await;
        let mut batch_stream = self.subscribe_batch().await;
        let mut forced_stream = self.subscribe_forced().await;

        // spawn Instatus monitor
        InstatusMonitor::new(
            self.clickhouse.clone(),
            self.incident_client.clone(),
            self.instatus_component_id.clone(),
            Duration::from_secs(30),
            Duration::from_secs(30),
            None,
            2,
        )
        .spawn();

        loop {
            tokio::select! {
                maybe_l1_header = l1_stream.next() => {
                    match maybe_l1_header {
                        Some(header) => {
                            self.handle_l1_header(header).await;
                        }
                        None => {
                            tracing::warn!("L1 header stream ended; re-subscribing…");
                            l1_stream = self.subscribe_l1().await;
                        }
                    }
                }
                maybe_l2_header = l2_stream.next() => {
                    match maybe_l2_header {
                        Some(header) => {
                            self.handle_l2_header(header).await;
                        }
                        None => {
                            tracing::warn!("L2 header stream ended; re-subscribing…");
                            l2_stream = self.subscribe_l2().await;
                        }
                    }
                }
                maybe_batch = batch_stream.next() => {
                    match maybe_batch {
                        Some(batch) => {
                            self.handle_batch_proposed(batch).await;
                        }
                        None => {
                            tracing::warn!("Batch proposed stream ended; re-subscribing…");
                            batch_stream = self.subscribe_batch().await;
                        }
                    }
                }
                maybe_fi = forced_stream.next() => {
                    match maybe_fi {
                        Some(fi) => {
                            self.handle_forced_inclusion(fi).await;
                        }
                        None => {
                            tracing::warn!("Forced inclusion stream ended; re-subscribing…");
                            forced_stream = self.subscribe_forced().await;
                        }
                    }
                }
                else => {
                    // This branch should ideally not be reached if streams always re-subscribe.
                    // If it is, it implies all streams terminated simultaneously and failed to re-subscribe,
                    // which would be an unexpected state.
                    tracing::error!("All event streams ended and failed to re-subscribe. This should not happen. Shutting down driver loop.");
                    break;
                }
            }
        }
        Ok(())
    }

    async fn handle_l1_header(&self, header: L1Header) {
        if let Err(e) = self.clickhouse.insert_l1_header(&header).await {
            tracing::error!(header_number = header.number, err = %e, "Failed to insert L1 header");
        } else {
            info!("Inserted L1 header: {:?}", header.number);
        }

        // TODO: uncomment this when this is deployed
        /*
        let opt_candidates = match self.extractor.get_operator_candidates_for_current_epoch().await {
            Ok(c) => Some(c),
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
        */
        let candidates: Vec<Address> = Vec::new();

        let opt_current_operator = match self.extractor.get_operator_for_current_epoch().await {
            Ok(op) => Some(op),
            Err(e) => {
                tracing::error!(block = header.number, err = %e, "get_operator_for_current_epoch failed");
                None
            }
        };

        let opt_next_operator = match self.extractor.get_operator_for_next_epoch().await {
            Ok(op) => Some(op),
            Err(e) => {
                // The first slot in the epoch doesn't have any next operator
                if header.slot % EPOCH_SLOTS != 0 {
                    tracing::error!(block = header.number, err = %e, "get_operator_for_next_epoch failed");
                }
                None
            }
        };

        if opt_current_operator.is_some() || opt_next_operator.is_some() {
            if let Err(e) = self
                .clickhouse
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
                info!("Inserted preconf data for slot: {:?}", header.slot);
            }
        } else {
            info!(
                "Skipping preconf data insertion for slot {:?} due to errors fetching operator data.",
                header.slot
            );
        }
    }

    async fn handle_l2_header(&mut self, header: L2Header) {
        // Detect reorgs
        // The simplified on_new_block now only takes the new block's number.
        // It returns Some(depth) if new_block_number < current_head_number.
        if let Some(depth) = self.reorg.on_new_block(header.number) {
            // The insert_l2_reorg function expects (block_number, depth)
            // The block_number should be the new head number after the reorg.
            if let Err(e) = self.clickhouse.insert_l2_reorg(header.number, depth).await {
                tracing::error!(block_number = header.number, depth = depth, err = %e, "Failed to insert L2 reorg");
            } else {
                info!("Inserted L2 reorg: new head {}, depth {}", header.number, depth);
            }
        } else if let Err(e) = self.clickhouse.insert_l2_header(&header).await {
            tracing::error!(block_number = header.number, err = %e, "Failed to insert L2 header");
        } else {
            info!("Inserted L2 header: {}", header.number);
        }
    }

    async fn handle_batch_proposed(&self, batch: BatchProposed) {
        if let Err(e) = self.clickhouse.insert_batch(&batch).await {
            tracing::error!(batch_last_block = ?batch.last_block_number(), err = %e, "Failed to insert batch");
        } else {
            info!("Inserted batch: {:?}", batch.last_block_number());
        }
    }

    async fn handle_forced_inclusion(&self, fi: ForcedInclusionProcessed) {
        if let Err(e) = self.clickhouse.insert_forced_inclusion(&fi).await {
            tracing::error!(blob_hash = ?fi.blobHash, err = %e, "Failed to insert forced inclusion");
        } else {
            info!("Inserted forced inclusion processed: {:?}", fi.blobHash);
        }
    }
}
