//! Taikoscope Driver

use std::time::Duration;

use eyre::Result;
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
    last_batch_slot: u64,
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
            batch_proof_timeout_secs: opts.instatus.batch_proof_timeout_secs,
            last_batch_slot: 0,
        })
    }

    /// Subscribe to the L1 header stream with a retry loop.
    ///
    /// The function keeps attempting to subscribe until a stream is
    /// successfully returned, waiting five seconds between retries.
    async fn subscribe_l1(&self) -> L1HeaderStream {
        loop {
            match self.extractor.get_l1_header_stream().await {
                Ok(s) => return s,
                Err(e) => {
                    tracing::error!(error = %e, "L1 subscribe failed, retrying in 5s");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    /// Subscribe to the L2 header stream with a retry loop.
    ///
    /// Similar to [`subscribe_l1`], this will retry every five seconds
    /// until a stream is obtained.
    async fn subscribe_l2(&self) -> L2HeaderStream {
        loop {
            match self.extractor.get_l2_header_stream().await {
                Ok(s) => return s,
                Err(e) => {
                    tracing::error!(error = %e, "L2 subscribe failed, retrying in 5s");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    /// Subscribe to `BatchProposed` events with a retry loop.
    async fn subscribe_batch(&self) -> BatchProposedStream {
        loop {
            match self.extractor.get_batch_proposed_stream().await {
                Ok(s) => return s,
                Err(e) => {
                    tracing::error!(error = %e, "BatchProposed subscribe failed, retrying in 5s");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    /// Subscribe to `ForcedInclusionProcessed` events with a retry loop.
    async fn subscribe_forced(&self) -> ForcedInclusionStream {
        loop {
            match self.extractor.get_forced_inclusion_stream().await {
                Ok(s) => return s,
                Err(e) => {
                    tracing::error!(error = %e, "ForcedInclusion subscribe failed, retrying in 5s");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    /// Subscribe to `BatchesProved` events with a retry loop.
    async fn subscribe_proved(&self) -> BatchesProvedStream {
        loop {
            match self.extractor.get_batches_proved_stream().await {
                Ok(s) => return s,
                Err(e) => {
                    tracing::error!(error = %e, "BatchesProved subscribe failed, retrying in 5s");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    /// Subscribe to `BatchesVerified` events with a retry loop.
    async fn subscribe_verified(&self) -> BatchesVerifiedStream {
        loop {
            match self.extractor.get_batches_verified_stream().await {
                Ok(s) => return s,
                Err(e) => {
                    tracing::error!(error = %e, "BatchesVerified subscribe failed, retrying in 5s");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    /// Consume the driver and drive the infinite processing loop.
    pub async fn start(mut self) -> Result<()> {
        info!("Starting event loop");

        let l1_stream = self.subscribe_l1().await;
        let l2_stream = self.subscribe_l2().await;
        let batch_stream = self.subscribe_batch().await;
        let forced_stream = self.subscribe_forced().await;
        let proved_stream = self.subscribe_proved().await;
        let verified_stream = self.subscribe_verified().await;

        self.spawn_monitors();

        self.event_loop(
            l1_stream,
            l2_stream,
            batch_stream,
            forced_stream,
            proved_stream,
            verified_stream,
        )
        .await
    }

    /// Spawn all background monitors used by the driver.
    ///
    /// Each monitor runs in its own task and reports incidents via the
    /// [`IncidentClient`].
    fn spawn_monitors(&self) {
        InstatusL1Monitor::new(
            self.clickhouse.clone(),
            self.incident_client.clone(),
            self.instatus_batch_component_id.clone(),
            Duration::from_secs(self.instatus_monitor_threshold_secs),
            Duration::from_secs(self.instatus_monitor_poll_interval_secs),
        )
        .spawn();

        InstatusMonitor::new(
            self.clickhouse.clone(),
            self.incident_client.clone(),
            self.instatus_l2_component_id.clone(),
            Duration::from_secs(self.instatus_monitor_threshold_secs),
            Duration::from_secs(self.instatus_monitor_poll_interval_secs),
        )
        .spawn();

        BatchProofTimeoutMonitor::new(
            self.clickhouse.clone(),
            self.incident_client.clone(),
            self.instatus_batch_proof_timeout_component_id.clone(),
            Duration::from_secs(self.batch_proof_timeout_secs),
            Duration::from_secs(60),
        )
        .spawn();

        BatchVerifyTimeoutMonitor::new(
            self.clickhouse.clone(),
            self.incident_client.clone(),
            self.instatus_batch_verify_timeout_component_id.clone(),
            Duration::from_secs(self.batch_proof_timeout_secs),
            Duration::from_secs(60),
        )
        .spawn();
    }

    /// Process incoming events from all subscriptions.
    ///
    /// The loop listens to every stream concurrently and delegates
    /// handling of each event type to the appropriate method. If any
    /// stream ends, it attempts to resubscribe before continuing.
    async fn event_loop(
        &mut self,
        mut l1_stream: L1HeaderStream,
        mut l2_stream: L2HeaderStream,
        mut batch_stream: BatchProposedStream,
        mut forced_stream: ForcedInclusionStream,
        mut proved_stream: BatchesProvedStream,
        mut verified_stream: BatchesVerifiedStream,
    ) -> Result<()> {
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
                maybe_proved = proved_stream.next() => {
                    match maybe_proved {
                        Some(proved) => {
                            self.handle_batches_proved(proved).await;
                        }
                        None => {
                            tracing::warn!("Batches proved stream ended; re-subscribing…");
                            proved_stream = self.subscribe_proved().await;
                        }
                    }
                }
                maybe_verified = verified_stream.next() => {
                    match maybe_verified {
                        Some(verified) => {
                            self.handle_batches_verified(verified).await;
                        }
                        None => {
                            tracing::warn!("Batches verified stream ended; re-subscribing…");
                            verified_stream = self.subscribe_verified().await;
                        }
                    }
                }
                else => {
                    tracing::error!("All event streams ended and failed to re-subscribe. Shutting down driver loop");
                    break;
                }
            }
        }
        Ok(())
    }

    /// Insert the received L1 header and related preconfirmation data.
    async fn handle_l1_header(&mut self, header: L1Header) {
        if let Err(e) = self.clickhouse.insert_l1_header(&header).await {
            tracing::error!(header_number = header.number, err = %e, "Failed to insert L1 header");
        } else {
            info!(header_number = header.number, "Inserted L1 header");
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

        if header.slot > self.last_batch_slot + 1 {
            let sequencer = opt_current_operator.unwrap_or(Address::ZERO);
            for s in (self.last_batch_slot + 1)..header.slot {
                match self.clickhouse.insert_missed_slot(sequencer, s, header.number).await {
                    Ok(_) => info!(slot = s, "Inserted missed slot"),
                    Err(e) => tracing::error!(slot = s, err = %e, "Failed to insert missed slot"),
                }
            }
            self.last_batch_slot = header.slot - 1;
        }

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
                info!(slot = header.slot, "Inserted preconf data for slot");
            }
        } else {
            info!(
                slot = header.slot,
                "Skipping preconf data insertion due to errors fetching operator data"
            );
        }
    }

    /// Process an L2 header event, inserting statistics and detecting reorgs.
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
                info!(new_head = header.number, depth, "Inserted L2 reorg");
            }
        } else {
            match self.extractor.get_l2_block_stats(header.number, header.base_fee_per_gas).await {
                Ok((sum_gas_used, sum_tx, sum_priority_fee)) => {
                    let event = clickhouse::L2HeadEvent {
                        l2_block_number: header.number,
                        block_hash: *header.hash,
                        block_ts: header.timestamp,
                        sum_gas_used,
                        sum_tx,
                        sum_priority_fee,
                        sequencer: header.beneficiary.into_array(),
                    };

                    if let Err(e) = self.clickhouse.insert_l2_header(&event).await {
                        tracing::error!(block_number = header.number, err = %e, "Failed to insert L2 header");
                    } else {
                        info!(l2_header = header.number, "Inserted L2 header");
                    }
                }
                Err(e) => {
                    tracing::error!(block_number = header.number, err = %e, "Failed to fetch block stats");
                }
            }
        }
    }

    /// Store a newly proposed batch.
    async fn handle_batch_proposed(&mut self, batch: BatchProposed) {
        if let Err(e) = self.clickhouse.insert_batch(&batch).await {
            tracing::error!(batch_last_block = ?batch.last_block_number(), err = %e, "Failed to insert batch");
        } else {
            info!(last_block_number = ?batch.last_block_number(), "Inserted batch");
            self.last_batch_slot = batch.meta.batchId;
        }
    }

    /// Record a forced inclusion event.
    async fn handle_forced_inclusion(&self, fi: ForcedInclusionProcessed) {
        if let Err(e) = self.clickhouse.insert_forced_inclusion(&fi).await {
            tracing::error!(blob_hash = ?fi.blobHash, err = %e, "Failed to insert forced inclusion");
        } else {
            info!(blob_hash = ?fi.blobHash, "Inserted forced inclusion processed");
        }
    }

    /// Store batches that have been proved on L1.
    async fn handle_batches_proved(&self, proved_data: (BatchesProved, u64)) {
        let (proved, l1_block_number) = proved_data;
        if let Err(e) = self.clickhouse.insert_proved_batch(&proved, l1_block_number).await {
            tracing::error!(batch_ids = ?proved.batch_ids_proved(), err = %e, "Failed to insert proved batch");
        } else {
            info!(batch_ids = ?proved.batch_ids_proved(), "Inserted proved batch");
        }
    }

    /// Store batches that have been verified on L1.
    async fn handle_batches_verified(&self, verified_data: (chainio::BatchesVerified, u64)) {
        let (verified, l1_block_number) = verified_data;
        if let Err(e) = self.clickhouse.insert_verified_batch(&verified, l1_block_number).await {
            tracing::error!(batch_id = ?verified.batch_id, err = %e, "Failed to insert verified batch");
        } else {
            info!(batch_id = ?verified.batch_id, "Inserted verified batch");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::Address;
    use clickhouse_rs::test::{Mock, handlers};
    use config::{ApiOpts, ClickhouseOpts, InstatusOpts, Opts, RpcOpts, TaikoAddressOpts};
    use futures::future;
    use http::StatusCode;
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

    #[tokio::test]
    async fn new_respects_batch_proof_timeout_from_opts() {
        // Mock ClickHouse server with enough handlers for `init_db`
        let mock = Mock::new();
        for _ in 0..11 {
            mock.add(handlers::failure(StatusCode::OK));
        }

        let url = Url::parse(mock.url()).unwrap();
        let (l1_url, l1_handle) = start_ws_server().await;
        let (l2_url, l2_handle) = start_ws_server().await;
        let opts = Opts {
            clickhouse: ClickhouseOpts {
                url,
                db: "test".into(),
                username: "user".into(),
                password: "pass".into(),
            },
            rpc: RpcOpts { l1_url, l2_url },
            api: ApiOpts { host: "127.0.0.1".into(), port: 3000 },
            taiko_addresses: TaikoAddressOpts {
                inbox_address: Address::ZERO,
                preconf_whitelist_address: Address::ZERO,
                taiko_wrapper_address: Address::ZERO,
            },
            instatus: InstatusOpts {
                api_key: "key".into(),
                page_id: "page".into(),
                batch_component_id: String::new(),
                batch_proof_timeout_component_id: String::new(),
                batch_verify_timeout_component_id: String::new(),
                l2_component_id: String::new(),
                monitor_poll_interval_secs: 30,
                monitor_threshold_secs: 96,
                batch_proof_timeout_secs: 999,
            },
            reset_db: false,
        };

        let driver = Driver::new(opts.clone()).await.unwrap();
        l1_handle.abort();
        l2_handle.abort();
        assert_eq!(driver.batch_proof_timeout_secs, opts.instatus.batch_proof_timeout_secs);
    }
}
