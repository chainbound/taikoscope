//! Taikoscope Extractor

use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use derive_more::Debug;
use eyre::Result;
use tokio_stream::StreamExt;

/// Extractor client
#[derive(Debug)]
pub struct Extractor {
    #[debug(skip)]
    provider: Box<dyn Provider + Send + Sync>,
}

impl Extractor {
    /// Create a new extractor
    pub async fn new(rpc_url: &str) -> Result<Self> {
        let ws = WsConnect::new(rpc_url);
        let provider = ProviderBuilder::new().connect_ws(ws).await?;

        Ok(Self { provider: Box::new(provider) })
    }

    /// Process blocks
    pub async fn process_blocks(&self) -> Result<()> {
        // Subscribe to new blocks
        let sub = self.provider.subscribe_blocks().await?;
        let mut stream = sub.into_stream();
        println!("Subscribed to block headers");

        while let Some(block) = stream.next().await {
            println!("Block: number {:?}", block.number);
        }

        Ok(())
    }
}
