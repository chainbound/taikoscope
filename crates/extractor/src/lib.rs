//! Taikoscope Extractor
use chainio::{
    self, DefaultProvider,
    ITaikoInbox::{BatchProposed, BatchesProved},
    taiko::{
        preconf_whitelist::TaikoPreconfWhitelist,
        wrapper::{ITaikoWrapper::ForcedInclusionProcessed, TaikoWrapper},
    },
};

use std::pin::Pin;

use alloy::{
    primitives::{Address, BlockHash, BlockNumber},
    providers::{Provider, ProviderBuilder},
};
use alloy_rpc_client::ClientBuilder;
use chainio::TaikoInbox;
use derive_more::Debug;
use eyre::Result;
use primitives::retries::{DEFAULT_RETRY_LAYER, RetryWsConnect};
use std::time::Duration;
use tokio::{sync::mpsc, time::sleep};
use tokio_stream::{Stream, StreamExt, wrappers::UnboundedReceiverStream};
use tracing::{error, info, warn};
use url::Url;

/// Extractor client
#[derive(Debug)]
pub struct Extractor {
    #[debug(skip)]
    l1_provider: DefaultProvider,
    #[debug(skip)]
    l2_provider: DefaultProvider,
    preconf_whitelist: TaikoPreconfWhitelist,
    taiko_inbox: TaikoInbox,
    taiko_wrapper: TaikoWrapper,
}

/// L1 Header
#[derive(Debug)]
pub struct L1Header {
    /// Block number
    pub number: u64,
    /// Block hash
    pub hash: BlockHash,
    /// Block slot
    pub slot: u64,
    /// Extracted block timestamp
    pub timestamp: u64,
}

/// L2 Header
#[derive(Debug)]
pub struct L2Header {
    /// Block number
    pub number: u64,
    /// Block hash
    pub hash: BlockHash,
    /// Block parent hash
    pub parent_hash: BlockHash,
    /// Block timestamp
    pub timestamp: u64,
    /// Gas used
    pub gas_used: u64,
    /// Beneficiary
    pub beneficiary: Address,
}

/// Stream of L1 headers
pub type L1HeaderStream = Pin<Box<dyn Stream<Item = L1Header> + Send>>;
/// Stream of L2 headers
pub type L2HeaderStream = Pin<Box<dyn Stream<Item = L2Header> + Send>>;
/// Stream of batch proposed events
pub type BatchProposedStream = Pin<Box<dyn Stream<Item = BatchProposed> + Send>>;
/// Stream of batches proved events
pub type BatchesProvedStream =
    Pin<Box<dyn Stream<Item = (chainio::ITaikoInbox::BatchesProved, u64)> + Send>>;
/// Stream of forced inclusion processed events
pub type ForcedInclusionStream = Pin<Box<dyn Stream<Item = ForcedInclusionProcessed> + Send>>;

impl Extractor {
    /// Create a new extractor
    pub async fn new(
        l1_rpc_url: Url,
        l2_rpc_url: Url,
        inbox_address: Address,
        preconf_whitelist_address: Address,
        taiko_wrapper_address: Address,
    ) -> Result<Self> {
        let l1_ws = RetryWsConnect::from_url(l1_rpc_url);
        let l1_client = ClientBuilder::default().layer(DEFAULT_RETRY_LAYER).pubsub(l1_ws).await?;
        let l1_provider = ProviderBuilder::new().connect_client(l1_client);

        let l2_ws = RetryWsConnect::from_url(l2_rpc_url);
        let l2_client = ClientBuilder::default().layer(DEFAULT_RETRY_LAYER).pubsub(l2_ws).await?;
        let l2_provider = ProviderBuilder::new().connect_client(l2_client);

        let taiko_inbox = TaikoInbox::new_readonly(inbox_address, l1_provider.clone());
        let preconf_whitelist =
            TaikoPreconfWhitelist::new_readonly(preconf_whitelist_address, l1_provider.clone());
        let taiko_wrapper = TaikoWrapper::new_readonly(taiko_wrapper_address, l1_provider.clone());

        Ok(Self { l1_provider, l2_provider, preconf_whitelist, taiko_inbox, taiko_wrapper })
    }

    /// Get a stream of L1 headers. This stream will attempt to automatically
    /// resubscribe and continue yielding headers in case of disconnections.
    pub async fn get_l1_header_stream(&self) -> Result<L1HeaderStream> {
        let (tx, rx) = mpsc::unbounded_channel();
        let provider = self.l1_provider.clone();

        tokio::spawn(async move {
            loop {
                info!("Attempting to subscribe to L1 block headers...");
                let sub_result = provider.subscribe_blocks().await;

                let mut block_stream = match sub_result {
                    Ok(sub) => {
                        info!("Successfully subscribed to L1 block headers.");
                        sub.into_stream()
                    }
                    Err(e) => {
                        error!("Failed to subscribe to L1 blocks: {}. Retrying in 5s...", e);
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };

                while let Some(block_data) = block_stream.next().await {
                    let header = L1Header {
                        number: block_data.number,
                        hash: block_data.hash,
                        // TODO: Get slot instead. For now, using block number as a placeholder.
                        slot: block_data.number,
                        timestamp: block_data.timestamp,
                    };
                    if tx.send(header).is_err() {
                        error!("L1 header receiver dropped. Stopping L1 header task.");
                        return; // Exit task if receiver is gone
                    }
                }
                warn!("L1 block stream ended. Attempting to resubscribe...");
                // Outer loop will retry subscription.
            }
        });

        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }

    /// Get a stream of L2 headers. This stream will attempt to automatically
    /// resubscribe and continue yielding headers in case of disconnections.
    pub async fn get_l2_header_stream(&self) -> Result<L2HeaderStream> {
        let (tx, rx) = mpsc::unbounded_channel();
        let provider = self.l2_provider.clone();

        tokio::spawn(async move {
            loop {
                info!("Attempting to subscribe to L2 block headers...");
                let sub_result = provider.subscribe_blocks().await;

                let mut block_stream = match sub_result {
                    Ok(sub) => {
                        info!("Successfully subscribed to L2 block headers.");
                        sub.into_stream()
                    }
                    Err(e) => {
                        error!("Failed to subscribe to L2 blocks: {}. Retrying in 5s...", e);
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };

                while let Some(block_data) = block_stream.next().await {
                    let header = L2Header {
                        number: block_data.number,
                        hash: block_data.hash,
                        parent_hash: block_data.parent_hash,
                        timestamp: block_data.timestamp,
                        gas_used: block_data.gas_used,
                        beneficiary: block_data.beneficiary,
                    };
                    if tx.send(header).is_err() {
                        error!("L2 header receiver dropped. Stopping L2 header task.");
                        return; // Exit task if receiver is gone
                    }
                }
                warn!("L2 block stream ended. Attempting to resubscribe...");
            }
        });

        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }

    /// Subscribes to the `TaikoInbox` `BatchProposed` event and returns a stream of decoded events.
    /// This stream will attempt to automatically resubscribe and continue yielding events.
    pub async fn get_batch_proposed_stream(&self) -> Result<BatchProposedStream> {
        let (tx, rx) = mpsc::unbounded_channel();
        let provider = self.l1_provider.clone();
        let taiko_inbox = self.taiko_inbox.clone(); // Clone for use in the spawned task

        tokio::spawn(async move {
            loop {
                info!("Attempting to subscribe to TaikoInbox BatchProposed events...");
                let filter = taiko_inbox.batch_proposed_filter();
                let sub_result = provider.subscribe_logs(&filter).await;

                let mut log_stream = match sub_result {
                    Ok(sub) => {
                        info!("Successfully subscribed to TaikoInbox BatchProposed events.");
                        sub.into_stream()
                    }
                    Err(e) => {
                        error!(
                            "Failed to subscribe to BatchProposed logs: {}. Retrying in 5s...",
                            e
                        );
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };

                while let Some(log) = log_stream.next().await {
                    match log.log_decode::<BatchProposed>() {
                        Ok(decoded) => {
                            if tx.send(decoded.data().clone()).is_err() {
                                error!(
                                    "BatchProposed receiver dropped. Stopping BatchProposed event task."
                                );
                                return; // Exit task if receiver is gone
                            }
                        }
                        Err(err) => {
                            warn!("Failed to decode BatchProposed log: {}", err);
                            // Optionally, decide if this is a critical error or can be skipped.
                            // For now, we just log and continue.
                        }
                    }
                }
                warn!("BatchProposed log stream ended. Attempting to resubscribe...");
            }
        });

        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }

    /// Subscribes to the `TaikoInbox` `BatchesProved` event and returns a stream of decoded events
    /// along with the block number. This stream will attempt to automatically resubscribe and
    /// continue yielding events.
    pub async fn get_batches_proved_stream(&self) -> Result<BatchesProvedStream> {
        let (tx, rx) = mpsc::unbounded_channel();
        let provider = self.l1_provider.clone();
        let taiko_inbox = self.taiko_inbox.clone(); // Clone for use in the spawned task

        tokio::spawn(async move {
            loop {
                info!("Attempting to subscribe to TaikoInbox BatchesProved events...");
                let filter = taiko_inbox.batches_proved_filter();
                let sub_result = provider.subscribe_logs(&filter).await;

                let mut log_stream = match sub_result {
                    Ok(sub) => {
                        info!("Successfully subscribed to TaikoInbox BatchesProved events.");
                        sub.into_stream()
                    }
                    Err(e) => {
                        error!(
                            "Failed to subscribe to BatchesProved logs: {}. Retrying in 5s...",
                            e
                        );
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };

                while let Some(log) = log_stream.next().await {
                    match log.log_decode::<BatchesProved>() {
                        Ok(decoded) => {
                            // Include the block number in the tuple
                            let l1_block_number = log.block_number.unwrap_or(0);
                            if tx.send((decoded.data().clone(), l1_block_number)).is_err() {
                                error!(
                                    "BatchesProved receiver dropped. Stopping BatchesProved event task."
                                );
                                return; // Exit task if receiver is gone
                            }
                        }
                        Err(err) => {
                            warn!("Failed to decode BatchesProved log: {}", err);
                            // Optionally, decide if this is a critical error or can be skipped.
                            // For now, we just log and continue.
                        }
                    }
                }
                warn!("BatchesProved log stream ended. Attempting to resubscribe...");
            }
        });

        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }

    /// Subscribes to the `TaikoWrapper` `ForcedInclusionProcessed` event and returns a stream of
    /// decoded events. This stream will attempt to automatically resubscribe and continue
    /// yielding events.
    pub async fn get_forced_inclusion_stream(&self) -> Result<ForcedInclusionStream> {
        let (tx, rx) = mpsc::unbounded_channel();
        let provider = self.l1_provider.clone();
        let taiko_wrapper = self.taiko_wrapper.clone(); // Clone for use in the spawned task

        tokio::spawn(async move {
            loop {
                info!("Attempting to subscribe to TaikoWrapper ForcedInclusionProcessed events...");
                let filter = taiko_wrapper.forced_inclusion_processed_filter();
                let sub_result = provider.subscribe_logs(&filter).await;

                let mut log_stream = match sub_result {
                    Ok(sub) => {
                        info!(
                            "Successfully subscribed to TaikoWrapper ForcedInclusionProcessed events."
                        );
                        sub.into_stream()
                    }
                    Err(e) => {
                        error!(
                            "Failed to subscribe to ForcedInclusionProcessed logs: {}. Retrying in 5s...",
                            e
                        );
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };

                while let Some(log) = log_stream.next().await {
                    match log.log_decode::<ForcedInclusionProcessed>() {
                        Ok(decoded) => {
                            if tx.send(decoded.data().clone()).is_err() {
                                error!(
                                    "ForcedInclusionProcessed receiver dropped. Stopping ForcedInclusionProcessed event task."
                                );
                                return; // Exit task if receiver is gone
                            }
                        }
                        Err(err) => {
                            warn!("Failed to decode ForcedInclusionProcessed log: {}", err);
                            // Optionally, decide if this is a critical error or can be skipped.
                        }
                    }
                }
                warn!("ForcedInclusionProcessed log stream ended. Attempting to resubscribe...");
            }
        });

        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }

    /// Get the current epoch operator
    pub async fn get_operator_for_current_epoch(&self) -> Result<Address> {
        let operator = self.preconf_whitelist.get_operator_for_current_epoch().await?;
        Ok(operator)
    }

    /// Get the next epoch operator
    pub async fn get_operator_for_next_epoch(&self) -> Result<Address> {
        let operator = self.preconf_whitelist.get_operator_for_next_epoch().await?;
        Ok(operator)
    }

    /// Get the operator candidates for the current epoch
    pub async fn get_operator_candidates_for_current_epoch(&self) -> Result<Vec<Address>> {
        let candidates = self.preconf_whitelist.get_operator_candidates_for_current_epoch().await?;
        Ok(candidates)
    }
}

/// Detects reorgs based on block numbers.
#[derive(Debug)]
pub struct ReorgDetector {
    head_number: BlockNumber,
}

impl ReorgDetector {
    /// Create a new reorg detector
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self { head_number: 0 }
    }

    /// Checks a new block number against the current head number.
    /// Returns the reorg depth if the new block number is less than the current head.
    /// Always updates the internal head number to the new block number.
    pub fn on_new_block(&mut self, new_block_number: BlockNumber) -> Option<u16> {
        // Assume no reorg
        let mut reorg_depth = None;

        // A reorg is detected if the new block's number is less than the current head's number.
        // This check also implies self.head_number must have been initialized (i.e., not 0).
        if new_block_number < self.head_number {
            // Depth is the number of blocks orphaned from the previous chain.
            // e.g. if old head was 10 and new head is 8, depth is 2 (blocks 10 and 9 are orphaned).
            let depth_val = self.head_number.saturating_sub(new_block_number);

            // Ensure a positive depth, then cap at u16::MAX.
            if depth_val > 0 {
                reorg_depth = Some(depth_val.min(u16::MAX as u64) as u16);
            }
        }

        // Always update the head to the new block number.
        self.head_number = new_block_number;

        reorg_depth
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_block() {
        let mut det = ReorgDetector::new();
        // First block received is 5. head_number is 0. 5 is not < 0. No reorg.
        assert_eq!(det.on_new_block(5), None);
        assert_eq!(det.head_number, 5);
    }

    #[test]
    fn subsequent_blocks_increasing() {
        let mut det = ReorgDetector::new();
        det.on_new_block(5); // head_number becomes 5
        // New block 6. 6 is not < 5. No reorg.
        assert_eq!(det.on_new_block(6), None);
        assert_eq!(det.head_number, 6);
        // New block 7. 7 is not < 6. No reorg.
        assert_eq!(det.on_new_block(7), None);
        assert_eq!(det.head_number, 7);
    }

    #[test]
    fn reorg_to_lower_number() {
        let mut det = ReorgDetector::new();
        det.on_new_block(10); // head_number is 10
        // New block 8. 8 < 10. Reorg. Depth = 10 - 8 = 2.
        assert_eq!(det.on_new_block(8), Some(2));
        assert_eq!(det.head_number, 8); // Head is updated to 8
    }

    #[test]
    fn reorg_by_one() {
        let mut det = ReorgDetector::new();
        det.on_new_block(10); // head_number is 10
        // New block 9. 9 < 10. Reorg. Depth = 10 - 9 = 1.
        assert_eq!(det.on_new_block(9), Some(1));
        assert_eq!(det.head_number, 9);
    }

    #[test]
    fn same_block_number_no_reorg() {
        let mut det = ReorgDetector::new();
        det.on_new_block(10); // head_number is 10
        // New block 10. 10 is not < 10. No reorg.
        assert_eq!(det.on_new_block(10), None);
        assert_eq!(det.head_number, 10); // Head is updated to 10 (no change)
    }

    #[test]
    fn reorg_depth_capped_at_u16_max() {
        let mut det = ReorgDetector::new();
        det.on_new_block(u16::MAX as u64 + 10);
        // New block 1. 1 < u16::MAX + 10. Reorg. Depth = u16::MAX + 10 - 1. Capped to u16::MAX.
        assert_eq!(det.on_new_block(1), Some(u16::MAX));
        assert_eq!(det.head_number, 1);
    }

    #[test]
    fn reorg_from_initial_zero_state() {
        let mut det = ReorgDetector::new(); // head_number is 0
        // New block 5. 5 is not < 0. No reorg.
        assert_eq!(det.on_new_block(5), None);
        assert_eq!(det.head_number, 5);
    }

    #[test]
    fn reorg_to_zero_not_possible_if_blocks_are_positive() {
        let mut det = ReorgDetector::new();
        det.on_new_block(5); // head_number is 5
        // New block 0. 0 < 5. Reorg. Depth = 5 - 0 = 5.
        assert_eq!(det.on_new_block(0), Some(5));
        assert_eq!(det.head_number, 0);
    }
}
