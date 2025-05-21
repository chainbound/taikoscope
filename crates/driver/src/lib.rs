//! Taikoscope Driver

mod event;

use std::time::Duration;

use eyre::Result;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio_stream::StreamExt;
use tracing::info;

use alloy_primitives::Address;
use chainio::{
    ITaikoInbox::{BatchProposed, BatchesProved},
    taiko::wrapper::ITaikoWrapper::ForcedInclusionProcessed,
};
use clickhouse::ClickhouseClient;
use config::Opts;
use extractor::{
    BatchProposedStream, BatchesProvedStream, BatchesVerifiedStream, Extractor,
    ForcedInclusionStream, L1Header, L1HeaderStream, L2Header, L2HeaderStream, ReorgDetector,
};
use incident::{
    BatchProofTimeoutMonitor, InstatusL1Monitor, InstatusMonitor, Monitor,
    client::Client as IncidentClient, monitor::BatchVerifyTimeoutMonitor,
};
use primitives::shutdown::{ShutdownSignal, run_until_shutdown};

use crate::event::DriverEvent;

/// An EPOCH is a series of 32 slots.
pub const EPOCH_SLOTS: u64 = 32;

/// Taikoscope Driver
#[derive(Debug)]
pub struct Driver {
    clickhouse: ClickhouseClient,
    extractor: Extractor,
    reorg: ReorgDetector,
    incident_client: IncidentClient,
    instatus_batch_component_id: String,
    instatus_batch_proof_timeout_component_id: String,
    instatus_batch_verify_timeout_component_id: String,
    instatus_l2_component_id: String,
    instatus_monitor_poll_interval_secs: u64,
    instatus_monitor_threshold_secs: u64,
    batch_proof_timeout_secs: u64,
}

impl Driver {
    /// Build everything (client, extractor, detector), but don't start the event loop yet.
    pub async fn new(opts: Opts) -> Result<Self> {
        // init db client
        let clickhouse = ClickhouseClient::new(
            opts.clickhouse.url,
            opts.clickhouse.db.clone(),
            opts.clickhouse.username.clone(),
            opts.clickhouse.password.clone(),
        )?;
        clickhouse.init_db(opts.reset_db).await?;

        // init extractor
        let extractor = Extractor::new(
            opts.rpc.l1_url,
            opts.rpc.l2_url,
            opts.taiko_addresses.inbox_address,
            opts.taiko_addresses.preconf_whitelist_address,
            opts.taiko_addresses.taiko_wrapper_address,
        )
        .await?;

        // init incident client and component IDs
        let instatus_batch_component_id = opts.instatus.batch_component_id.clone();
        let instatus_batch_proof_timeout_component_id =
            opts.instatus.batch_proof_timeout_component_id.clone();
        let instatus_batch_verify_timeout_component_id =
            opts.instatus.batch_verify_timeout_component_id.clone();
        let instatus_l2_component_id = opts.instatus.l2_component_id.clone();
        let incident_client =
            IncidentClient::new(opts.instatus.api_key.clone(), opts.instatus.page_id.clone());

        Ok(Self {
            clickhouse,
            extractor,
            reorg: ReorgDetector::new(),
            incident_client,
            instatus_batch_component_id,
            instatus_batch_proof_timeout_component_id,
            instatus_batch_verify_timeout_component_id,
            instatus_l2_component_id,
            instatus_monitor_poll_interval_secs: opts.instatus.monitor_poll_interval_secs,
            instatus_monitor_threshold_secs: opts.instatus.monitor_threshold_secs,
            batch_proof_timeout_secs: 3 * 60 * 60, // 3 hours in seconds
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

    async fn subscribe_proved(&self) -> BatchesProvedStream {
        loop {
            match self.extractor.get_batches_proved_stream().await {
                Ok(s) => return s,
                Err(e) => {
                    tracing::error!("BatchesProved subscribe failed: {}. Retrying in 5s…", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn subscribe_verified(&self) -> BatchesVerifiedStream {
        loop {
            match self.extractor.get_batches_verified_stream().await {
                Ok(s) => return s,
                Err(e) => {
                    tracing::error!("BatchesVerified subscribe failed: {}. Retrying in 5s…", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    fn spawn_l1_task(&self, tx: UnboundedSender<DriverEvent>) {
        let extractor = self.extractor.clone();
        tokio::spawn(async move {
            loop {
                let mut stream = loop {
                    match extractor.get_l1_header_stream().await {
                        Ok(s) => break s,
                        Err(e) => {
                            tracing::error!("L1 subscribe failed: {}. Retrying in 5s…", e);
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                    }
                };

                while let Some(header) = stream.next().await {
                    if tx.send(DriverEvent::L1Header(header)).is_err() {
                        tracing::error!("Driver event receiver dropped. Stopping L1 listener.");
                        return;
                    }
                }

                tracing::warn!("L1 header stream ended. Re-subscribing...");
            }
        });
    }

    fn spawn_l2_task(&self, tx: UnboundedSender<DriverEvent>) {
        let extractor = self.extractor.clone();
        tokio::spawn(async move {
            loop {
                let mut stream = loop {
                    match extractor.get_l2_header_stream().await {
                        Ok(s) => break s,
                        Err(e) => {
                            tracing::error!("L2 subscribe failed: {}. Retrying in 5s…", e);
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                    }
                };

                while let Some(header) = stream.next().await {
                    if tx.send(DriverEvent::L2Header(header)).is_err() {
                        tracing::error!("Driver event receiver dropped. Stopping L2 listener.");
                        return;
                    }
                }

                tracing::warn!("L2 header stream ended. Re-subscribing...");
            }
        });
    }

    fn spawn_batch_task(&self, tx: UnboundedSender<DriverEvent>) {
        let extractor = self.extractor.clone();
        tokio::spawn(async move {
            loop {
                let mut stream = loop {
                    match extractor.get_batch_proposed_stream().await {
                        Ok(s) => break s,
                        Err(e) => {
                            tracing::error!(
                                "BatchProposed subscribe failed: {}. Retrying in 5s…",
                                e
                            );
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                    }
                };

                while let Some(batch) = stream.next().await {
                    if tx.send(DriverEvent::BatchProposed(batch)).is_err() {
                        tracing::error!("Driver event receiver dropped. Stopping Batch listener.");
                        return;
                    }
                }

                tracing::warn!("Batch proposed stream ended. Re-subscribing...");
            }
        });
    }

    fn spawn_forced_task(&self, tx: UnboundedSender<DriverEvent>) {
        let extractor = self.extractor.clone();
        tokio::spawn(async move {
            loop {
                let mut stream = loop {
                    match extractor.get_forced_inclusion_stream().await {
                        Ok(s) => break s,
                        Err(e) => {
                            tracing::error!(
                                "ForcedInclusion subscribe failed: {}. Retrying in 5s…",
                                e
                            );
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                    }
                };

                while let Some(event) = stream.next().await {
                    if tx.send(DriverEvent::ForcedInclusion(event)).is_err() {
                        tracing::error!(
                            "Driver event receiver dropped. Stopping ForcedInclusion listener."
                        );
                        return;
                    }
                }

                tracing::warn!("Forced inclusion stream ended. Re-subscribing...");
            }
        });
    }

    fn spawn_proved_task(&self, tx: UnboundedSender<DriverEvent>) {
        let extractor = self.extractor.clone();
        tokio::spawn(async move {
            loop {
                let mut stream = loop {
                    match extractor.get_batches_proved_stream().await {
                        Ok(s) => break s,
                        Err(e) => {
                            tracing::error!(
                                "BatchesProved subscribe failed: {}. Retrying in 5s…",
                                e
                            );
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                    }
                };

                while let Some(proved) = stream.next().await {
                    if tx.send(DriverEvent::BatchesProved(proved)).is_err() {
                        tracing::error!(
                            "Driver event receiver dropped. Stopping BatchesProved listener."
                        );
                        return;
                    }
                }

                tracing::warn!("Batches proved stream ended. Re-subscribing...");
            }
        });
    }

    fn spawn_verified_task(&self, tx: UnboundedSender<DriverEvent>) {
        let extractor = self.extractor.clone();
        tokio::spawn(async move {
            loop {
                let mut stream = loop {
                    match extractor.get_batches_verified_stream().await {
                        Ok(s) => break s,
                        Err(e) => {
                            tracing::error!(
                                "BatchesVerified subscribe failed: {}. Retrying in 5s…",
                                e
                            );
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                    }
                };

                while let Some(verified) = stream.next().await {
                    if tx.send(DriverEvent::BatchesVerified(verified)).is_err() {
                        tracing::error!(
                            "Driver event receiver dropped. Stopping BatchesVerified listener."
                        );
                        return;
                    }
                }

                tracing::warn!("Batches verified stream ended. Re-subscribing...");
            }
        });
    }

    /// Consume the driver and drive the infinite processing loop.
    pub async fn start(mut self) -> Result<()> {
        info!("Starting event loop...");

        let (tx, rx) = unbounded_channel();

        self.spawn_l1_task(tx.clone());
        self.spawn_l2_task(tx.clone());
        self.spawn_batch_task(tx.clone());
        self.spawn_forced_task(tx.clone());
        self.spawn_proved_task(tx.clone());
        self.spawn_verified_task(tx);

        // spawn Instatus batch monitor
        InstatusL1Monitor::new(
            self.clickhouse.clone(),
            self.incident_client.clone(),
            self.instatus_batch_component_id.clone(),
            Duration::from_secs(self.instatus_monitor_threshold_secs),
            Duration::from_secs(self.instatus_monitor_poll_interval_secs),
        )
        .spawn();

        // spawn Instatus L2 head monitor
        InstatusMonitor::new(
            self.clickhouse.clone(),
            self.incident_client.clone(),
            self.instatus_l2_component_id.clone(),
            Duration::from_secs(self.instatus_monitor_threshold_secs),
            Duration::from_secs(self.instatus_monitor_poll_interval_secs),
        )
        .spawn();

        // spawn batch proof timeout monitor (checks if batches take >3h to prove)
        BatchProofTimeoutMonitor::new(
            self.clickhouse.clone(),
            self.incident_client.clone(),
            self.instatus_batch_proof_timeout_component_id.clone(),
            Duration::from_secs(self.batch_proof_timeout_secs),
            Duration::from_secs(60), // Run every minute
        )
        .spawn();

        // spawn batch verify timeout monitor (checks if batches take >3h to verify)
        BatchVerifyTimeoutMonitor::new(
            self.clickhouse.clone(),
            self.incident_client.clone(),
            self.instatus_batch_verify_timeout_component_id.clone(),
            Duration::from_secs(self.batch_proof_timeout_secs),
            Duration::from_secs(60), // Run every minute
        )
        .spawn();

        let shutdown = ShutdownSignal::new();
        run_until_shutdown(self.event_loop(rx), shutdown, || info!("Shutdown signal received"))
            .await
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

    async fn handle_batches_proved(&self, proved_data: (BatchesProved, u64)) {
        let (proved, l1_block_number) = proved_data;
        if let Err(e) = self.clickhouse.insert_proved_batch(&proved, l1_block_number).await {
            tracing::error!(batch_ids = ?proved.batch_ids_proved(), err = %e, "Failed to insert proved batch");
        } else {
            info!("Inserted proved batch: batch_ids={:?}", proved.batch_ids_proved());
        }
    }

    async fn handle_batches_verified(&self, verified_data: (chainio::BatchesVerified, u64)) {
        let (verified, l1_block_number) = verified_data;
        if let Err(e) = self.clickhouse.insert_verified_batch(&verified, l1_block_number).await {
            tracing::error!(batch_id = ?verified.batch_id, err = %e, "Failed to insert verified batch");
        } else {
            info!("Inserted verified batch: batch_id={:?}", verified.batch_id);
        }
    }

    async fn event_loop(&mut self, mut rx: UnboundedReceiver<DriverEvent>) -> Result<()> {
        while let Some(event) = rx.recv().await {
            match event {
                DriverEvent::L1Header(h) => self.handle_l1_header(h).await,
                DriverEvent::L2Header(h) => self.handle_l2_header(h).await,
                DriverEvent::BatchProposed(b) => self.handle_batch_proposed(b).await,
                DriverEvent::ForcedInclusion(fi) => self.handle_forced_inclusion(fi).await,
                DriverEvent::BatchesProved(p) => self.handle_batches_proved(p).await,
                DriverEvent::BatchesVerified(v) => self.handle_batches_verified(v).await,
            }
        }
        Ok(())
    }
}
