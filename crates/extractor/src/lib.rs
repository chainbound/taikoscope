//! Taikoscope Extractor

use std::pin::Pin;

use alloy::{
    primitives::BlockHash,
    providers::{Provider, ProviderBuilder, WsConnect},
};
use derive_more::Debug;
use eyre::Result;
use tokio_stream::{Stream, StreamExt};

/// Extractor client
#[derive(Debug)]
pub struct Extractor {
    #[debug(skip)]
    provider: Box<dyn Provider + Send + Sync>,
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
    pub async fn new(rpc_url: &str) -> Result<Self> {
        let ws = WsConnect::new(rpc_url);
        let provider = ProviderBuilder::new().connect_ws(ws).await?;

        Ok(Self { provider: Box::new(provider) })
    }

    /// Get a stream of blocks from the provider
    pub async fn get_block_stream(&self) -> Result<Pin<Box<dyn Stream<Item = Block> + Send>>> {
        // Subscribe to new blocks
        let sub = self.provider.subscribe_blocks().await?;
        let stream = sub.into_stream();
        println!("Subscribed to block headers");

        // Convert stream to block stream
        let block_stream = stream.map(|raw_block| Block {
            number: raw_block.number,
            hash: raw_block.hash,
            slot: raw_block.number, // TODO: Get slot instead
            timestamp: raw_block.timestamp,
        });

        Ok(Box::pin(block_stream))
    }
}
