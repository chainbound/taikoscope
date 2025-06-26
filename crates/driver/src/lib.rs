//! Taikoscope Driver

use std::time::Duration;

use alloy_primitives::{Address, B256};
use eyre::Result;
use network::public_rpc_monitor;
use tokio_stream::StreamExt;
use tracing::info;
use url::Url;

use chainio::{
    ITaikoInbox::{BatchProposed, BatchesProved},
    taiko::wrapper::ITaikoWrapper::ForcedInclusionProcessed,
};
use clickhouse::{AddressBytes, ClickhouseReader, ClickhouseWriter, HashBytes};
use config::Opts;
use extractor::{
    BatchProposedStream, BatchesProvedStream, BatchesVerifiedStream, Extractor,
    ForcedInclusionStream, ReorgDetector,
};
use incident::{
    BatchProofTimeoutMonitor, InstatusL1Monitor, InstatusMonitor, Monitor,
    client::Client as IncidentClient, monitor::BatchVerifyTimeoutMonitor,
};
use primitives::headers::{L1Header, L1HeaderStream, L2Header, L2HeaderStream};

/// An EPOCH is a series of 32 slots.
pub const EPOCH_SLOTS: u64 = 32;

/// Taikoscope Driver
#[derive(Debug)]
pub struct Driver {
    clickhouse: ClickhouseWriter,
    clickhouse_reader: ClickhouseReader,
    extractor: Extractor,
    reorg: ReorgDetector,
    incident_client: IncidentClient,
    instatus_batch_submission_component_id: String,
    instatus_proof_submission_component_id: String,
    instatus_proof_verification_component_id: String,
    instatus_transaction_sequencing_component_id: String,
    instatus_monitors_enabled: bool,
    instatus_monitor_poll_interval_secs: u64,
    instatus_monitor_threshold_secs: u64,
    batch_proof_timeout_secs: u64,
    last_proposed_l2_block: u64,
    last_l2_header: Option<(u64, Address)>,
    public_rpc_url: Option<Url>,
}

impl Driver {
    /// Build everything (client, extractor, detector), but don't start the event loop yet.
    pub async fn new(opts: Opts) -> Result<Self> {
        Self::new_with_migrations(opts, true).await
    }

    /// Build everything with option to skip database migrations (useful for tests)
    pub async fn new_with_migrations(opts: Opts, run_migrations: bool) -> Result<Self> {
        // verify monitoring configuration before doing any heavy work
        if opts.instatus.monitors_enabled && !opts.instatus.enabled() {
            return Err(eyre::eyre!(
                "Instatus configuration missing; set the INSTATUS_* environment variables"
            ));
        }

        if !opts.instatus.monitors_enabled {
            info!("Instatus monitors disabled; no incidents will be reported");
        }

        // init db client
        let clickhouse = ClickhouseWriter::new(
            opts.clickhouse.url.clone(),
            opts.clickhouse.db.clone(),
            opts.clickhouse.username.clone(),
            opts.clickhouse.password.clone(),
        );

        // init db reader for monitors
        let clickhouse_reader = ClickhouseReader::new(
            opts.clickhouse.url,
            opts.clickhouse.db.clone(),
            opts.clickhouse.username.clone(),
            opts.clickhouse.password.clone(),
        )?;

        if run_migrations {
            info!("🚀 Running database migrations...");
            clickhouse.init_db(opts.reset_db).await?;
            info!("✅ Database migrations completed");
        }

        // init extractor
        let extractor = Extractor::new(
            opts.rpc.l1_url,
            opts.rpc.l2_url,
            opts.taiko_addresses.inbox_address,
            opts.taiko_addresses.preconf_whitelist_address,
            opts.taiko_addresses.taiko_wrapper_address,
        )
        .await?;

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
            clickhouse,
            clickhouse_reader,
            extractor,
            reorg: ReorgDetector::new(),
            incident_client,
            instatus_batch_submission_component_id,
            instatus_proof_submission_component_id,
            instatus_proof_verification_component_id,
            instatus_transaction_sequencing_component_id,
            instatus_monitors_enabled: opts.instatus.monitors_enabled,
            instatus_monitor_poll_interval_secs: opts.instatus.monitor_poll_interval_secs,
            instatus_monitor_threshold_secs: opts.instatus.monitor_threshold_secs,
            batch_proof_timeout_secs: opts.instatus.batch_proof_timeout_secs,
            last_proposed_l2_block: 0,
            last_l2_header: None,
            public_rpc_url: opts.rpc.public_url,
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
        if let Some(url) = &self.public_rpc_url {
            tracing::info!(url = url.as_str(), "public rpc monitor enabled");
            public_rpc_monitor::spawn_public_rpc_monitor(url.clone());
        }

        if !self.instatus_monitors_enabled {
            return;
        }

        InstatusL1Monitor::new(
            self.clickhouse_reader.clone(),
            self.incident_client.clone(),
            self.instatus_batch_submission_component_id.clone(),
            Duration::from_secs(self.instatus_monitor_threshold_secs),
            Duration::from_secs(self.instatus_monitor_poll_interval_secs),
        )
        .spawn();

        InstatusMonitor::new(
            self.clickhouse_reader.clone(),
            self.incident_client.clone(),
            self.instatus_transaction_sequencing_component_id.clone(),
            Duration::from_secs(self.instatus_monitor_threshold_secs),
            Duration::from_secs(self.instatus_monitor_poll_interval_secs),
        )
        .spawn();

        BatchProofTimeoutMonitor::new(
            self.clickhouse_reader.clone(),
            self.incident_client.clone(),
            self.instatus_proof_submission_component_id.clone(),
            Duration::from_secs(self.batch_proof_timeout_secs),
            Duration::from_secs(60),
        )
        .spawn();

        BatchVerifyTimeoutMonitor::new(
            self.clickhouse_reader.clone(),
            self.incident_client.clone(),
            self.instatus_proof_verification_component_id.clone(),
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
                        Some(batch_data) => {
                            self.handle_batch_proposed(batch_data).await;
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
    async fn handle_l1_header(&self, header: L1Header) {
        if let Err(e) = self.clickhouse.insert_l1_header(&header).await {
            tracing::error!(header_number = header.number, err = %e, "Failed to insert L1 header");
        } else {
            info!(header_number = header.number, "Inserted L1 header");
        }

        let opt_candidates = match self.extractor.get_operator_candidates_for_current_epoch().await
        {
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
                tracing::error!(block = header.number, err = %e, "get_operator_for_current_epoch failed");
                None
            }
        };

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
        self.last_l2_header = Some((header.number, header.beneficiary));
        // Detect reorgs
        // It returns Some(depth) if new_block_number < current_head_number.
        if let Some(depth) = self.reorg.on_new_block(header.number) {
            // The block_number should be the new head number after the reorg.
            if let Err(e) = self.clickhouse.insert_l2_reorg(header.number, depth).await {
                tracing::error!(block_number = header.number, depth = depth, err = %e, "Failed to insert L2 reorg");
            } else {
                info!(new_head = header.number, depth, "Inserted L2 reorg");
            }
        } else {
            match self.extractor.get_l2_block_stats(header.number, header.base_fee_per_gas).await {
                Ok((sum_gas_used, sum_tx, sum_priority_fee)) => {
                    let sum_base_fee =
                        sum_gas_used.saturating_mul(header.base_fee_per_gas.unwrap_or(0) as u128);
                    let event = clickhouse::L2HeadEvent {
                        l2_block_number: header.number,
                        block_hash: HashBytes(*header.hash),
                        block_ts: header.timestamp,
                        sum_gas_used,
                        sum_tx,
                        sum_priority_fee,
                        sum_base_fee,
                        sequencer: AddressBytes(header.beneficiary.into_array()),
                    };

                    if let Err(e) = self.clickhouse.insert_l2_header(&event).await {
                        tracing::error!(block_number = header.number, err = %e, "Failed to insert L2 header");
                    } else {
                        info!(
                            l2_header = header.number,
                            block_ts = header.timestamp,
                            "Inserted L2 header"
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(block_number = header.number, err = %e, "Failed to fetch block stats");
                }
            }
        }
    }

    /// Store a newly proposed batch.
    async fn handle_batch_proposed(&mut self, batch_data: (BatchProposed, B256)) {
        let (batch, l1_tx_hash) = batch_data;

        if let Err(e) = self.clickhouse.insert_batch(&batch).await {
            tracing::error!(batch_last_block = ?batch.last_block_number(), err = %e, "Failed to insert batch");
        } else {
            info!(last_block_number = ?batch.last_block_number(), "Inserted batch");
        }
        self.last_proposed_l2_block = batch.last_block_number();

        // Skip L1 data cost calculation for zero hash (test scenarios)
        if l1_tx_hash.is_zero() {
            tracing::debug!("Skipping L1 data cost calculation for zero transaction hash");
            return;
        }

        // Calculate L1 data cost from the transaction that proposed this batch
        match self.extractor.get_receipt(l1_tx_hash).await {
            Ok(receipt) => {
                let cost = primitives::l1_data_cost::cost_from_receipt(&receipt);
                if let Err(e) = self
                    .clickhouse
                    .insert_l1_data_cost(batch.info.proposedIn, self.last_proposed_l2_block, cost)
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
            Err(e) => {
                tracing::error!(
                    tx_hash = ?l1_tx_hash,
                    err = %e,
                    "Failed to fetch receipt for batch proposal transaction"
                );
            }
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
    async fn handle_batches_proved(&self, proved_data: (BatchesProved, u64, B256)) {
        let (proved, l1_block_number, tx_hash) = proved_data;
        if let Err(e) = self.clickhouse.insert_proved_batch(&proved, l1_block_number).await {
            tracing::error!(
                batch_ids = ?proved.batch_ids_proved(),
                err = %e,
                "Failed to insert proved batch"
            );
        } else {
            info!(batch_ids = ?proved.batch_ids_proved(), "Inserted proved batch");
        }

        if tx_hash.is_zero() {
            tracing::debug!("Skipping prove cost calculation for zero transaction hash");
            return;
        }

        match self.extractor.get_receipt(tx_hash).await {
            Ok(receipt) => {
                let cost = primitives::l1_data_cost::cost_from_receipt(&receipt);
                let cost_per_batch =
                    Self::calculate_cost_per_batch(cost, proved.batch_ids_proved().len());
                for batch_id in proved.batch_ids_proved() {
                    if let Err(e) = self
                        .clickhouse
                        .insert_prove_cost(l1_block_number, *batch_id, cost_per_batch)
                        .await
                    {
                        tracing::error!(
                            l1_block_number,
                            batch_id,
                            tx_hash = ?tx_hash,
                            err = %e,
                            "Failed to insert prove cost"
                        );
                    } else {
                        info!(
                            l1_block_number,
                            batch_id,
                            tx_hash = ?tx_hash,
                            cost = cost_per_batch,
                            "Inserted prove cost"
                        );
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    tx_hash = ?tx_hash,
                    err = %e,
                    "Failed to fetch receipt for batch prove transaction"
                );
            }
        }
    }

    /// Store batches that have been verified on L1.
    async fn handle_batches_verified(&self, verified_data: (chainio::BatchesVerified, u64, B256)) {
        let (verified, l1_block_number, tx_hash) = verified_data;
        if let Err(e) = self.clickhouse.insert_verified_batch(&verified, l1_block_number).await {
            tracing::error!(batch_id = ?verified.batch_id, err = %e, "Failed to insert verified batch");
        } else {
            info!(batch_id = ?verified.batch_id, "Inserted verified batch");
        }

        if tx_hash.is_zero() {
            tracing::debug!("Skipping verify cost calculation for zero transaction hash");
            return;
        }

        match self.extractor.get_receipt(tx_hash).await {
            Ok(receipt) => {
                let cost = primitives::l1_data_cost::cost_from_receipt(&receipt);
                if let Err(e) = self
                    .clickhouse
                    .insert_verify_cost(l1_block_number, verified.batch_id, cost)
                    .await
                {
                    tracing::error!(
                        l1_block_number,
                        batch_id = ?verified.batch_id,
                        tx_hash = ?tx_hash,
                        err = %e,
                        "Failed to insert verify cost"
                    );
                } else {
                    info!(
                        l1_block_number,
                        batch_id = ?verified.batch_id,
                        tx_hash = ?tx_hash,
                        cost,
                        "Inserted verify cost"
                    );
                }
            }
            Err(e) => {
                tracing::error!(
                    tx_hash = ?tx_hash,
                    err = %e,
                    "Failed to fetch receipt for batch verify transaction"
                );
            }
        }
    }

    const fn calculate_cost_per_batch(total_cost: u128, num_batches: usize) -> u128 {
        if num_batches == 0 { 0 } else { total_cost / num_batches as u128 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chainio::{ITaikoInbox, taiko::wrapper::ITaikoWrapper};
    use clickhouse::{
        AddressBytes, BatchBlockRow, BatchRow, ForcedInclusionProcessedRow, HashBytes,
        ProvedBatchRow, VerifiedBatchRow,
    };
    use clickhouse_rs::test::{Mock, handlers};
    use config::{
        ApiOpts, ClickhouseOpts, InstatusOpts, NatsOpts, Opts, RpcOpts, TaikoAddressOpts,
    };

    use alloy_primitives::{Address, B256};
    use futures::future;
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
            nats: NatsOpts {
                nats_url: Some(nats_url),
                username: Some("natsuser".into()),
                password: Some("natspass".into()),
            },
            rpc: RpcOpts { l1_url, l2_url, public_url: None },
            api: ApiOpts {
                host: "127.0.0.1".into(),
                port: 3000,
                allowed_origins: Vec::new(),
                rate_limit_max_requests: 1000,
                rate_limit_period_secs: 60,
            },
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
                monitors_enabled: true,
                monitor_poll_interval_secs: 30,
                monitor_threshold_secs: 96,
                batch_proof_timeout_secs: 999,
            },
            reset_db: false,
        }
    }

    #[tokio::test]
    async fn new_respects_batch_proof_timeout_from_opts() {
        // Mock ClickHouse server - minimal setup since we skip migrations
        let mock = Mock::new();
        // Just one handler since we're not actually doing any DB operations

        let url = Url::parse(mock.url()).unwrap();
        let (l1_url, l1_handle) = start_ws_server().await;
        let (l2_url, l2_handle) = start_ws_server().await;
        let nats_url = Url::parse("nats://localhost:4222").unwrap();
        let opts = make_opts(url, nats_url, l1_url.clone(), l2_url.clone());

        let driver = Driver::new_with_migrations(opts.clone(), false).await.unwrap();
        l1_handle.abort();
        l2_handle.abort();
        assert_eq!(driver.batch_proof_timeout_secs, opts.instatus.batch_proof_timeout_secs);
    }

    #[tokio::test]
    async fn handle_batch_proposed_inserts_row() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<BatchRow>());
        // Add handler for batch_blocks table insert
        let _ctl_blocks = mock.add(handlers::record::<BatchBlockRow>());

        let url = Url::parse(mock.url()).unwrap();
        let (l1_url, l1_handle) = start_ws_server().await;
        let (l2_url, l2_handle) = start_ws_server().await;
        let nats_url = Url::parse("nats://localhost:4222").unwrap();
        let mut driver = Driver::new_with_migrations(
            make_opts(url, nats_url, l1_url.clone(), l2_url.clone()),
            false,
        )
        .await
        .unwrap();
        l1_handle.abort();
        l2_handle.abort();

        let batch = ITaikoInbox::BatchProposed {
            info: ITaikoInbox::BatchInfo {
                proposedIn: 2,
                blobByteSize: 50,
                blocks: vec![ITaikoInbox::BlockParams::default(); 1],
                blobHashes: vec![B256::repeat_byte(1)],
                lastBlockId: 100, // Adding test value for last block ID
                ..Default::default()
            },
            meta: ITaikoInbox::BatchMetadata {
                proposer: Address::repeat_byte(2),
                batchId: 7,
                ..Default::default()
            },
            ..Default::default()
        };

        driver.handle_batch_proposed((batch, B256::ZERO)).await;

        let rows: Vec<BatchRow> = ctl.collect().await;
        assert_eq!(
            rows,
            vec![BatchRow {
                l1_block_number: 2,
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
    async fn handle_forced_inclusion_inserts_row() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<ForcedInclusionProcessedRow>());

        let url = Url::parse(mock.url()).unwrap();
        let (l1_url, l1_handle) = start_ws_server().await;
        let (l2_url, l2_handle) = start_ws_server().await;
        let nats_url = Url::parse("nats://localhost:4222").unwrap();
        let driver = Driver::new_with_migrations(
            make_opts(url, nats_url, l1_url.clone(), l2_url.clone()),
            false,
        )
        .await
        .unwrap();
        l1_handle.abort();
        l2_handle.abort();

        let event = ITaikoWrapper::ForcedInclusionProcessed {
            blobHash: B256::repeat_byte(5),
            feeInGwei: 1,
            createdAtBatchId: 0,
            blobByteOffset: 0,
            blobByteSize: 0,
            blobCreatedIn: 0,
        };

        driver.handle_forced_inclusion(event).await;

        let rows: Vec<ForcedInclusionProcessedRow> = ctl.collect().await;
        assert_eq!(
            rows,
            vec![ForcedInclusionProcessedRow { blob_hash: HashBytes::from([5u8; 32]) }]
        );
    }

    #[tokio::test]
    async fn handle_batches_proved_inserts_rows() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<ProvedBatchRow>());

        let url = Url::parse(mock.url()).unwrap();
        let (l1_url, l1_handle) = start_ws_server().await;
        let (l2_url, l2_handle) = start_ws_server().await;
        let nats_url = Url::parse("nats://localhost:4222").unwrap();
        let driver = Driver::new_with_migrations(
            make_opts(url, nats_url, l1_url.clone(), l2_url.clone()),
            false,
        )
        .await
        .unwrap();
        l1_handle.abort();
        l2_handle.abort();

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

        driver.handle_batches_proved((proved, 10, B256::ZERO)).await;

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
    async fn handle_batches_verified_inserts_row() {
        let mock = Mock::new();
        let ctl = mock.add(handlers::record::<VerifiedBatchRow>());

        let url = Url::parse(mock.url()).unwrap();
        let (l1_url, l1_handle) = start_ws_server().await;
        let (l2_url, l2_handle) = start_ws_server().await;
        let nats_url = Url::parse("nats://localhost:4222").unwrap();
        let driver = Driver::new_with_migrations(
            make_opts(url, nats_url, l1_url.clone(), l2_url.clone()),
            false,
        )
        .await
        .unwrap();
        l1_handle.abort();
        l2_handle.abort();

        let verified = chainio::BatchesVerified { batch_id: 3, block_hash: [9u8; 32] };

        driver.handle_batches_verified((verified, 12, B256::ZERO)).await;

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

    #[test]
    fn calculate_cost_per_batch_even_split() {
        let cost = Driver::calculate_cost_per_batch(100, 4);
        assert_eq!(cost, 25);
    }

    #[test]
    fn calculate_cost_per_batch_rounds_down() {
        let cost = Driver::calculate_cost_per_batch(10, 3);
        assert_eq!(cost, 3);
    }
}
