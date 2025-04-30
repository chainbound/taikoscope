//! Taikoscope Extractor
use chainio::{self, ITaikoInbox::BatchProposed};

use std::pin::Pin;

use alloy::{
    primitives::{Address, BlockHash},
    providers::{Provider, ProviderBuilder, WsConnect},
    rpc::types::eth::Log,
};
use chainio::TaikoInbox;
use derive_more::Debug;
use eyre::Result;
use tokio_stream::{Stream, StreamExt};
use tracing::info;

/// Extractor client
#[derive(Debug)]
pub struct Extractor {
    #[debug(skip)]
    l1_provider: Box<dyn Provider + Send + Sync>,
    #[debug(skip)]
    l2_provider: Box<dyn Provider + Send + Sync>,
    taiko_inbox: TaikoInbox,
}

/// Block
#[derive(Debug)]
pub struct Block {
    /// Block number
    pub number: u64,
    /// Block hash
    pub hash: BlockHash,
    /// Block slot
    pub slot: u64,
    /// Extracted block timestamp
    pub timestamp: u64,
}

impl Extractor {
    /// Create a new extractor
    pub async fn new(l1_rpc_url: &str, l2_rpc_url: &str, inbox_address: Address) -> Result<Self> {
        let l1_el = WsConnect::new(l1_rpc_url);
        let l2_el = WsConnect::new(l2_rpc_url);
        let l1_provider = ProviderBuilder::new().connect_ws(l1_el).await?;
        let l2_provider = ProviderBuilder::new().connect_ws(l2_el).await?;

        let taiko_inbox = TaikoInbox::new_readonly(inbox_address, l1_provider.clone());

        Ok(Self {
            l1_provider: Box::new(l1_provider),
            l2_provider: Box::new(l2_provider),
            taiko_inbox,
        })
    }

    /// Get a stream of L1 blocks
    pub async fn get_l1_block_stream(&self) -> Result<Pin<Box<dyn Stream<Item = Block> + Send>>> {
        // Subscribe to new blocks
        let sub = self.l1_provider.subscribe_blocks().await?;
        let stream = sub.into_stream();
        info!("Subscribed to L1 block headers");

        // Convert stream to block stream
        let block_stream = stream.map(|raw_block| Block {
            number: raw_block.number,
            hash: raw_block.hash,
            slot: raw_block.number, // TODO: Get slot instead
            timestamp: raw_block.timestamp,
        });

        Ok(Box::pin(block_stream))
    }

    /// Get a stream of L2 blocks
    pub async fn get_l2_block_stream(&self) -> Result<Pin<Box<dyn Stream<Item = Block> + Send>>> {
        // Subscribe to new blocks
        let sub = self.l2_provider.subscribe_blocks().await?;
        let stream = sub.into_stream();
        info!("Subscribed to L2 block headers");

        // Convert stream to block stream
        let block_stream = stream.map(|raw_block| Block {
            number: raw_block.number,
            hash: raw_block.hash,
            slot: raw_block.number, // TODO: Get slot instead
            timestamp: raw_block.timestamp,
        });

        Ok(Box::pin(block_stream))
    }

    /// Subscribes to the `TaikoInbox`  `BatchProposed` event and returns a stream of decoded
    /// events.
    pub async fn get_batch_proposed_stream(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = BatchProposed> + Send>>> {
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

        Ok(Box::pin(batch_proposed_stream))
    }
}
