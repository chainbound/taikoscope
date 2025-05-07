//! Entrypoint.

use driver::Driver;

use clap::Parser;
use config::Opts;
use dotenvy::dotenv;
use tracing::info;

/// An EPOCH is a series of 32 slots.
pub const EPOCH_SLOTS: u64 = 32;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    if let Ok(custom_env_file) = std::env::var("ENV_FILE") {
        dotenvy::from_filename(custom_env_file)?;
    } else {
        // Try the default .env file, and ignore if it doesn't exist.
        dotenv().ok();
    }

    let opts = Opts::parse();
    info!("🔭 Taikoscope engine starting...");

    Driver::new(opts).await?.startup_sync().await?.start().await
}
