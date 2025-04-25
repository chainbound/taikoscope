//! Taikoscope Extractor

use std::error::Error;

use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use tokio_stream::StreamExt;

/// Extract blocks from the Ethereum blockchain
pub fn extractor() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let rpc_url = "wss://eth.merkle.io";
        extract_blocks(rpc_url).await.unwrap();
    })
}

async fn extract_blocks(rpc_url: &str) -> Result<(), Box<dyn Error>> {
    // Create a provider
    let ws = WsConnect::new(rpc_url);
    let provider = ProviderBuilder::new().connect_ws(ws).await?;

    // Subscribe to new blocks
    let sub = provider.subscribe_blocks().await?;
    let mut stream = sub.into_stream();
    println!("Subscribed to block headers");

    while let Some(block) = stream.next().await {
        println!("Block: number {:?}", block.number);
    }

    Ok(())
}
