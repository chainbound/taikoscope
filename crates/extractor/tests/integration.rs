//! Integration tests for the Extractor

use std::{
    process::{Child, Command},
    thread::sleep,
    time::Duration,
};

use alloy::primitives::address;
use extractor::{Extractor, L1Header};

use eyre::Result;
use tokio_stream::StreamExt;
use url::Url;

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

    let ws: Url = Url::parse("ws://127.0.0.1:8545").unwrap();

    // Create Extractor
    let ext = Extractor::new(
        ws.clone(),
        ws,
        address!("0xa7B208DE7F35E924D59C2b5f7dE3bb346E8A138C"),
        address!("0x3ea351Db28A9d4833Bf6c519F52766788DE14eC1"),
    )
    .await?;
    let mut stream = ext.get_l1_header_stream().await?;

    // Wait for the first block
    let header: L1Header = stream.next().await.expect("stream ended unexpectedly");
    assert!(header.number > 0);
    assert!(header.timestamp > 0);
    Ok(())
}
