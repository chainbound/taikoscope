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
    provider: Box<dyn Provider + Send + Sync>,
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
    pub async fn new(rpc_url: &str, inbox_address: Address) -> Result<Self> {
        let ws = WsConnect::new(rpc_url);
        let provider = ProviderBuilder::new().connect_ws(ws).await?;

        let taiko_inbox = TaikoInbox::new_readonly(inbox_address, provider.clone());

        Ok(Self { provider: Box::new(provider), taiko_inbox })
    }

    /// Get a stream of blocks from the provider
    pub async fn get_block_stream(&self) -> Result<Pin<Box<dyn Stream<Item = Block> + Send>>> {
        // Subscribe to new blocks
        let sub = self.provider.subscribe_blocks().await?;
        let stream = sub.into_stream();
        info!("Subscribed to block headers");

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
        let logs = self.provider.subscribe_logs(&filter).await?.into_stream();

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
