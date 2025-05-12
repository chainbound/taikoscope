//! Taikoscope Driver

use eyre::Result;
use tokio_stream::StreamExt;
use tracing::info;

use clickhouse::ClickhouseClient;
use config::Opts;
use extractor::{Extractor, ReorgDetector};
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
        let incident_client = IncidentClient::new(
            opts.instatus_api_key.clone(),
            opts.instatus_page_id.clone(),
            instatus_component_id.clone(),
        );

        Ok(Self {
            clickhouse,
            extractor,
            reorg: ReorgDetector::new(),
            incident_client,
            instatus_component_id,
        })
    }

    /// Consume the driver and drive the infinite processing loop.
    pub async fn start(mut self) -> Result<()> {
        info!("Starting event loop...");

        let mut l1_stream = self.extractor.get_l1_header_stream().await?;
        let mut l2_stream = self.extractor.get_l2_header_stream().await?;
        let mut batch_stream = self.extractor.get_batch_proposed_stream().await?;
        let mut forced_stream = self.extractor.get_forced_inclusion_stream().await?;

        // spawn Instatus monitor
        InstatusMonitor::new(
            self.clickhouse.clone(),
            self.incident_client.clone(),
            self.instatus_component_id.clone(),
        )
        .spawn();

        loop {
            tokio::select! {
                Some(header) = l1_stream.next() => {
                    self.clickhouse.insert_l1_header(&header).await?;
                    info!("Inserted L1 header: {:?}", header.number);

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
                    let candidates = Vec::new();

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
                        self.clickhouse.insert_preconf_data(header.slot, candidates, opt_current_operator, opt_next_operator).await?;
                        info!("Inserted preconf data for slot: {:?}", header.slot);
                    } else {
                        info!("Skipping preconf data insertion for slot {:?} due to errors fetching operator data.", header.slot);
                    }
                }
                Some(header) = l2_stream.next() => {
                    // Detect reorgs
                    if let Some((hash, old_hash, depth)) = self.reorg.on_new_block(header.number, header.hash, header.parent_hash) {
                        self.clickhouse.insert_l2_reorg(header.number, hash, old_hash, depth).await?;
                        info!("Inserted L2 reorg: {:?}", header.number);
                    } else {
                        self.clickhouse.insert_l2_header(&header).await?;
                        info!("Inserted L2 header: {:?}", header.number);
                    }
                }
                Some(batch) = batch_stream.next() => {
                    self.clickhouse.insert_batch(&batch).await?;
                    info!("Inserted batch: {:?}", batch.last_block_number());
                }
                Some(fi) = forced_stream.next() => {
                    self.clickhouse.insert_forced_inclusion(&fi).await?;
                    info!("Inserted forced inclusion processed: {:?}", fi.forcedInclusion.blobHash);
                }
            }
        }
    }
}
