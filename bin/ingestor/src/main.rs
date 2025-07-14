#![allow(missing_docs)]

use clap::Parser;
use config::Opts;
use dotenvy::dotenv;
use driver::ingestor::IngestorDriver;
use runtime::shutdown::{ShutdownSignal, run_until_shutdown};
use tracing::info;
use tracing_subscriber::filter::EnvFilter;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    if let Ok(custom_env_file) = std::env::var("ENV_FILE") {
        dotenvy::from_filename(custom_env_file)?;
    } else {
        dotenv().ok();
    }

    let opts = Opts::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("Starting Taikoscope Ingestor");

    let driver = IngestorDriver::new(opts).await?;

    let shutdown_signal = ShutdownSignal::new();
    let on_shutdown = || {
        info!("Ingestor shutting down...");
    };

    run_until_shutdown(async move { driver.start().await }, shutdown_signal, on_shutdown).await
}
