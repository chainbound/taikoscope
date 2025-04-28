//! Integration tests for the Extractor

use std::{
    process::{Child, Command},
    thread::sleep,
    time::Duration,
};

use extractor::{Block, Extractor};

use eyre::Result;
use tokio_stream::StreamExt;

const WS_URL: &str = "ws://127.0.0.1:8545";

/// Spawn Anvil as a child process (auto-mining every second),
/// and kill it when dropped.
struct Anvil(Child);

impl Anvil {
    fn new() -> Result<Self> {
        let child = Command::new("anvil").args(["--port", "8545", "--block-time", "1"]).spawn()?;
        Ok(Self(child))
    }
}

impl Drop for Anvil {
    fn drop(&mut self) {
        self.0.kill().unwrap();
    }
}

#[tokio::test]
async fn test_get_block_stream() -> Result<()> {
    // Spawn Anvil
    let _anvil = Anvil::new()?;
    // Give it some time to start
    sleep(Duration::from_millis(500));

    // Create Extractor
    let ext = Extractor::new(WS_URL).await?;
    let mut stream = ext.get_block_stream().await?;

    // Wait for the first block
    let block: Block = stream.next().await.expect("stream ended unexpectedly");
    assert!(block.number > 0);
    assert!(block.timestamp > 0);
    Ok(())
}
