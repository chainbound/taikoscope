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
use alloy_consensus::BlockHeader;
use alloy_network_primitives::ReceiptResponse;
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
    /// Available L1 RPC URLs
    l1_urls: Vec<Url>,
    /// Available L2 RPC URLs
    l2_urls: Vec<Url>,
    inbox_address: Address,
    preconf_whitelist_address: Address,
    taiko_wrapper_address: Address,
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
    /// Base fee per gas
    pub base_fee_per_gas: Option<u64>,
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
/// Stream of batches verified events
pub type BatchesVerifiedStream =
    Pin<Box<dyn Stream<Item = (chainio::BatchesVerified, u64)> + Send>>;
/// Stream of forced inclusion processed events
pub type ForcedInclusionStream = Pin<Box<dyn Stream<Item = ForcedInclusionProcessed> + Send>>;

async fn connect_provider(url: &Url) -> Result<DefaultProvider> {
    let ws = RetryWsConnect::from_url(url.clone());
    let client = ClientBuilder::default().layer(DEFAULT_RETRY_LAYER).pubsub(ws).await?;
    Ok(ProviderBuilder::new().connect_client(client))
}

async fn connect_any(urls: &[Url]) -> Result<(DefaultProvider, Url)> {
    let mut last_err = None;
    for url in urls {
        match connect_provider(url).await {
            Ok(p) => return Ok((p, url.clone())),
            Err(e) => {
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap_or_else(|| eyre::eyre!("no rpc urls provided")))
}

impl Extractor {
    /// Create a new extractor
    pub async fn new(
        l1_rpc_urls: Vec<Url>,
        l2_rpc_urls: Vec<Url>,
        inbox_address: Address,
        preconf_whitelist_address: Address,
        taiko_wrapper_address: Address,
    ) -> Result<Self> {
        Ok(Self {
            l1_urls: l1_rpc_urls,
            l2_urls: l2_rpc_urls,
            inbox_address,
            preconf_whitelist_address,
            taiko_wrapper_address,
        })
    }

    /// Get a stream of L1 headers. This stream will attempt to automatically
    /// resubscribe and continue yielding headers in case of disconnections.
    pub async fn get_l1_header_stream(&self) -> Result<L1HeaderStream> {
        let (tx, rx) = mpsc::unbounded_channel();
        let urls = self.l1_urls.clone();

        tokio::spawn(async move {
            loop {
                let (provider, url) = match connect_any(&urls).await {
                    Ok(p) => p,
                    Err(e) => {
                        error!(error = %e, "Failed to connect to any L1 provider, retrying in 5s");
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };
                info!(url = %url, "Attempting to subscribe to L1 block headers...");
                let sub_result = provider.subscribe_blocks().await;

                let mut block_stream = match sub_result {
                    Ok(sub) => {
                        info!(url = %url, "Successfully subscribed to L1 block headers.");
                        sub.into_stream()
                    }
                    Err(e) => {
                        error!(error = %e, url = %url, "Failed to subscribe to L1 blocks, retrying in 5s");
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
            }
        });

        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }

    /// Get a stream of L2 headers. This stream will attempt to automatically
    /// resubscribe and continue yielding headers in case of disconnections.
    pub async fn get_l2_header_stream(&self) -> Result<L2HeaderStream> {
        let (tx, rx) = mpsc::unbounded_channel();
        let urls = self.l2_urls.clone();

        tokio::spawn(async move {
            loop {
                let (provider, url) = match connect_any(&urls).await {
                    Ok(p) => p,
                    Err(e) => {
                        error!(error = %e, "Failed to connect to any L2 provider, retrying in 5s");
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };
                info!(url = %url, "Attempting to subscribe to L2 block headers...");
                let sub_result = provider.subscribe_blocks().await;

                let mut block_stream = match sub_result {
                    Ok(sub) => {
                        info!(url = %url, "Successfully subscribed to L2 block headers.");
                        sub.into_stream()
                    }
                    Err(e) => {
                        error!(error = %e, url = %url, "Failed to subscribe to L2 blocks, retrying in 5s");
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
                        base_fee_per_gas: block_data.base_fee_per_gas(),
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
        let urls = self.l1_urls.clone();
        let inbox = self.inbox_address;

        tokio::spawn(async move {
            loop {
                let (provider, url) = match connect_any(&urls).await {
                    Ok(p) => p,
                    Err(e) => {
                        error!(error = %e, "Failed to connect to any L1 provider, retrying in 5s");
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };
                let taiko_inbox = TaikoInbox::new_readonly(inbox, provider.clone());
                info!(url = %url, "Attempting to subscribe to TaikoInbox BatchProposed events...");
                let filter = taiko_inbox.batch_proposed_filter();
                let sub_result = provider.subscribe_logs(&filter).await;

                let mut log_stream = match sub_result {
                    Ok(sub) => {
                        info!("Successfully subscribed to TaikoInbox BatchProposed events.");
                        sub.into_stream()
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to subscribe to BatchProposed logs, retrying in 5s");
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
                            warn!(error = %err, "Failed to decode BatchProposed log");
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
        let urls = self.l1_urls.clone();
        let inbox = self.inbox_address;

        tokio::spawn(async move {
            loop {
                let (provider, url) = match connect_any(&urls).await {
                    Ok(p) => p,
                    Err(e) => {
                        error!(error = %e, "Failed to connect to any L1 provider, retrying in 5s");
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };
                let taiko_inbox = TaikoInbox::new_readonly(inbox, provider.clone());
                info!(url = %url, "Attempting to subscribe to TaikoInbox BatchesProved events...");
                let filter = taiko_inbox.batches_proved_filter();
                let sub_result = provider.subscribe_logs(&filter).await;

                let mut log_stream = match sub_result {
                    Ok(sub) => {
                        info!("Successfully subscribed to TaikoInbox BatchesProved events.");
                        sub.into_stream()
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to subscribe to BatchesProved logs, retrying in 5s");
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
                            warn!(error = %err, "Failed to decode BatchesProved log");
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
        let urls = self.l1_urls.clone();
        let wrapper_addr = self.taiko_wrapper_address;

        tokio::spawn(async move {
            loop {
                let (provider, url) = match connect_any(&urls).await {
                    Ok(p) => p,
                    Err(e) => {
                        error!(error = %e, "Failed to connect to any L1 provider, retrying in 5s");
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };
                let taiko_wrapper = TaikoWrapper::new_readonly(wrapper_addr, provider.clone());
                info!(url = %url, "Attempting to subscribe to TaikoWrapper ForcedInclusionProcessed events...");
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
                        error!(error = %e, "Failed to subscribe to ForcedInclusionProcessed logs, retrying in 5s");
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
                            warn!(error = %err, "Failed to decode ForcedInclusionProcessed log");
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
        let (provider, _) = connect_any(&self.l1_urls).await?;
        let whitelist =
            TaikoPreconfWhitelist::new_readonly(self.preconf_whitelist_address, provider);
        Ok(whitelist.get_operator_for_current_epoch().await?)
    }

    /// Get the next epoch operator
    pub async fn get_operator_for_next_epoch(&self) -> Result<Address> {
        let (provider, _) = connect_any(&self.l1_urls).await?;
        let whitelist =
            TaikoPreconfWhitelist::new_readonly(self.preconf_whitelist_address, provider);
        Ok(whitelist.get_operator_for_next_epoch().await?)
    }

    /// Subscribes to the `TaikoInbox` `BatchesVerified` event and returns a stream of decoded
    /// events along with the block number. This stream will attempt to automatically
    /// resubscribe and continue yielding events.
    pub async fn get_batches_verified_stream(&self) -> Result<BatchesVerifiedStream> {
        let (tx, rx) = mpsc::unbounded_channel();
        let urls = self.l1_urls.clone();
        let inbox = self.inbox_address;

        tokio::spawn(async move {
            loop {
                let (provider, url) = match connect_any(&urls).await {
                    Ok(p) => p,
                    Err(e) => {
                        error!(error = %e, "Failed to connect to any L1 provider, retrying in 5s");
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };
                let taiko_inbox = TaikoInbox::new_readonly(inbox, provider.clone());
                info!(url = %url, "Attempting to subscribe to TaikoInbox BatchesVerified events...");
                let filter = taiko_inbox.batches_verified_filter();
                let sub_result = provider.subscribe_logs(&filter).await;

                let mut log_stream = match sub_result {
                    Ok(sub) => {
                        info!("Successfully subscribed to TaikoInbox BatchesVerified events.");
                        sub.into_stream()
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to subscribe to BatchesVerified logs, retrying in 5s");
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };

                while let Some(log) = log_stream.next().await {
                    // Extract batch_id from the correct position in the data
                    // For non-indexed uint64, it's in the last 8 bytes of the first 32-byte chunk
                    let batch_id = if log.data().data.len() >= 32 {
                        u64::from_be_bytes(log.data().data[24..32].try_into().unwrap_or([0u8; 8]))
                    } else {
                        0
                    };

                    // Extract the block hash - it's the entire second 32-byte chunk
                    let mut block_hash = [0u8; 32];
                    if log.data().data.len() >= 64 {
                        block_hash.copy_from_slice(&log.data().data[32..64]);
                    }

                    let verified = chainio::BatchesVerified { batch_id, block_hash };

                    // Include the block number in the tuple
                    let l1_block_number = log.block_number.unwrap_or(0);
                    if tx.send((verified, l1_block_number)).is_err() {
                        error!(
                            "BatchesVerified receiver dropped. Stopping BatchesVerified event task."
                        );
                        return; // Exit task if receiver is gone
                    }
                }
                warn!("BatchesVerified log stream ended. Attempting to resubscribe...");
            }
        });

        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }

    /// Get the operator candidates for the current epoch
    pub async fn get_operator_candidates_for_current_epoch(&self) -> Result<Vec<Address>> {
        let (provider, _) = connect_any(&self.l1_urls).await?;
        let whitelist =
            TaikoPreconfWhitelist::new_readonly(self.preconf_whitelist_address, provider);
        Ok(whitelist.get_operator_candidates_for_current_epoch().await?)
    }

    /// Calculate aggregated statistics for an L2 block by fetching its receipts.
    pub async fn get_l2_block_stats(
        &self,
        block_number: u64,
        base_fee: Option<u64>,
    ) -> Result<(u128, u32, u128)> {
        use alloy_rpc_types_eth::{BlockId, BlockNumberOrTag};

        let (provider, _) = connect_any(&self.l2_urls).await?;
        let block = BlockId::Number(BlockNumberOrTag::Number(block_number));
        let receipts_opt = provider.get_block_receipts(block).await?;
        let receipts = receipts_opt.ok_or_else(|| eyre::eyre!("missing receipts"))?;

        Ok(compute_block_stats(&receipts, base_fee))
    }
}

/// Compute aggregated gas and priority fee statistics for a set of receipts.
pub fn compute_block_stats<R: ReceiptResponse>(
    receipts: &[R],
    base_fee: Option<u64>,
) -> (u128, u32, u128) {
    let base = base_fee.unwrap_or(0) as u128;
    let mut sum_gas_used: u128 = 0;
    let mut sum_priority_fee: u128 = 0;

    for receipt in receipts {
        let gas = receipt.gas_used() as u128;
        sum_gas_used += gas;
        let priority_per_gas = receipt.effective_gas_price().saturating_sub(base);
        sum_priority_fee += priority_per_gas.saturating_mul(gas);
    }

    let tx_count = receipts.len() as u32;
    (sum_gas_used, tx_count, sum_priority_fee)
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
    use alloy::primitives::{B256, TxHash};

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

    #[derive(Clone, Copy)]
    struct TestReceipt {
        gas: u64,
        price: u128,
    }

    impl ReceiptResponse for TestReceipt {
        fn contract_address(&self) -> Option<Address> {
            None
        }
        fn status(&self) -> bool {
            true
        }
        fn block_hash(&self) -> Option<BlockHash> {
            None
        }
        fn block_number(&self) -> Option<u64> {
            None
        }
        fn transaction_hash(&self) -> TxHash {
            TxHash::ZERO
        }
        fn transaction_index(&self) -> Option<u64> {
            None
        }
        fn gas_used(&self) -> u64 {
            self.gas
        }
        fn effective_gas_price(&self) -> u128 {
            self.price
        }
        fn blob_gas_used(&self) -> Option<u64> {
            None
        }
        fn blob_gas_price(&self) -> Option<u128> {
            None
        }
        fn from(&self) -> Address {
            Address::ZERO
        }
        fn to(&self) -> Option<Address> {
            None
        }
        fn cumulative_gas_used(&self) -> u64 {
            self.gas
        }
        fn state_root(&self) -> Option<B256> {
            None
        }
    }

    #[test]
    fn compute_block_stats_basic() {
        let receipts =
            vec![TestReceipt { gas: 100, price: 10 }, TestReceipt { gas: 200, price: 20 }];
        let (gas, count, priority) = compute_block_stats(&receipts, Some(5));
        assert_eq!(gas, 300);
        assert_eq!(count, 2);
        assert_eq!(priority, 3500);
    }

    #[test]
    fn compute_block_stats_zero_base_fee() {
        let receipts = vec![TestReceipt { gas: 150, price: 40 }];
        let (gas, count, priority) = compute_block_stats(&receipts, None);
        assert_eq!(gas, 150);
        assert_eq!(count, 1);
        assert_eq!(priority, 6000);
    }
}
