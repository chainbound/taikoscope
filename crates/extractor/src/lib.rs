//! Taikoscope Extractor
use chainio::{
    self, DefaultProvider,
    ITaikoInbox::BatchProposed,
    taiko::{
        preconf_whitelist::TaikoPreconfWhitelist,
        wrapper::{ITaikoWrapper::ForcedInclusionProcessed, TaikoWrapper},
    },
};

use std::{collections::VecDeque, pin::Pin};

use alloy::{
    primitives::{Address, BlockHash, BlockNumber},
    providers::{Provider, ProviderBuilder, WsConnect},
    rpc::types::eth::Log,
};
use chainio::TaikoInbox;
use derive_more::Debug;
use eyre::Result;
use tokio_stream::{Stream, StreamExt};
use tracing::info;
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
        let l1_el = WsConnect::new(l1_rpc_url);
        let l2_el = WsConnect::new(l2_rpc_url);
        let l1_provider = ProviderBuilder::new().connect_ws(l1_el).await?;
        let l2_provider = ProviderBuilder::new().connect_ws(l2_el).await?;

        let taiko_inbox = TaikoInbox::new_readonly(inbox_address, l1_provider.clone());
        let preconf_whitelist =
            TaikoPreconfWhitelist::new_readonly(preconf_whitelist_address, l1_provider.clone());
        let taiko_wrapper = TaikoWrapper::new_readonly(taiko_wrapper_address, l1_provider.clone());

        Ok(Self { l1_provider, l2_provider, preconf_whitelist, taiko_inbox, taiko_wrapper })
    }

    /// Get a stream of L1 headers
    pub async fn get_l1_header_stream(&self) -> Result<L1HeaderStream> {
        // Subscribe to new blocks
        let sub = self.l1_provider.subscribe_blocks().await?;
        let stream = sub.into_stream();

        // Convert stream to header stream
        let header_stream = stream.map(|header| L1Header {
            number: header.number,
            hash: header.hash,
            slot: header.number, // TODO: Get slot instead
            timestamp: header.timestamp,
        });

        info!("Subscribed to L1 block headers");
        Ok(Box::pin(header_stream))
    }

    /// Get a stream of L2 headers
    pub async fn get_l2_header_stream(&self) -> Result<L2HeaderStream> {
        // Subscribe to new blocks
        let sub = self.l2_provider.subscribe_blocks().await?;
        let stream = sub.into_stream();

        // Convert stream to header stream
        let header_stream = stream.map(|header| L2Header {
            number: header.number,
            hash: header.hash,
            parent_hash: header.parent_hash,
            timestamp: header.timestamp,
            gas_used: header.gas_used,
            beneficiary: header.beneficiary,
        });

        info!("Subscribed to L2 block headers");
        Ok(Box::pin(header_stream))
    }

    /// Subscribes to the `TaikoInbox`  `BatchProposed` event and returns a stream of decoded
    /// events.
    pub async fn get_batch_proposed_stream(&self) -> Result<BatchProposedStream> {
        let filter = self.taiko_inbox.batch_proposed_filter();
        let logs = self.l1_provider.subscribe_logs(&filter).await?.into_stream();

        // Convert stream to batch proposed stream
        let batch_proposed_stream =
            logs.filter_map(|log: Log| match log.log_decode::<BatchProposed>() {
                Ok(decoded) => {
                    // Extract the BatchProposed event from the Log<BatchProposed>
                    Some(decoded.data().clone())
                }
                Err(err) => {
                    tracing::warn!("Failed to decode log: {}", err);
                    None
                }
            });

        info!("Subscribed to TaikoInbox BatchProposed events");

        Ok(Box::pin(batch_proposed_stream))
    }

    /// Subscribes to the `TaikoWrapper` `ForcedInclusionProcessed` event and returns a stream of
    /// decoded events.
    pub async fn get_forced_inclusion_stream(&self) -> Result<ForcedInclusionStream> {
        let filter = self.taiko_wrapper.forced_inclusion_processed_filter();
        let logs = self.l1_provider.subscribe_logs(&filter).await?.into_stream();

        // Convert stream to forced inclusion processed stream
        let forced_inclusion_processed_stream =
            logs.filter_map(|log: Log| match log.log_decode::<ForcedInclusionProcessed>() {
                Ok(decoded) => Some(decoded.data().clone()),
                Err(err) => {
                    tracing::warn!("Failed to decode log: {}", err);
                    None
                }
            });

        info!("Subscribed to TaikoWrapper ForcedInclusionProcessed events");
        Ok(Box::pin(forced_inclusion_processed_stream))
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

/// Detects reorgs
/// Stores the last 256 blocks in a buffer
#[derive(Debug)]
pub struct ReorgDetector {
    head_number: BlockNumber,
    head_hash: BlockHash,
    buf: [BlockHash; 256],
}

impl ReorgDetector {
    /// Create a new reorg detector
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self { head_number: 0, head_hash: BlockHash::ZERO, buf: [BlockHash::ZERO; 256] }
    }

    /// Returns info about the reorg if there is a reorg
    pub fn on_new_block(
        &mut self,
        number: BlockNumber,
        hash: BlockHash,
        parent: BlockHash,
    ) -> Option<(BlockHash, BlockHash, u8)> {
        // First block ever
        if self.head_number == 0 {
            self.head_number = number;
            self.head_hash = hash;
            self.buf[(number % 256) as usize] = hash;
            return None;
        }

        let idx = (number % 256) as usize;
        let old_hash = self.buf[idx];

        // Normal case, extend current head
        if number == self.head_number + 1 && parent == self.head_hash {
            self.head_number = number;
            self.head_hash = hash;
            self.buf[idx] = hash;
            return None;
        }

        // Reorg detected
        let depth = (self.head_number - number + 1).max(1) as u8;
        self.head_number = number;
        self.head_hash = hash;
        self.buf[idx] = hash;

        Some((hash, old_hash, depth))
    }
}
