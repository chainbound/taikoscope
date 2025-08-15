#![allow(missing_docs)]

use std::time::Duration;

use clap::Parser;
use config::Opts;
use dotenvy::dotenv;
use driver::driver::Driver;
use runtime::shutdown::{ShutdownSignal, run_until_shutdown_graceful};
use tokio::sync::broadcast;
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
        .with_file(true)
        .with_line_number(true)
        .with_target(true)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("Starting Taikoscope Binary");

    let driver = Driver::new(opts).await?;

    // Create broadcast channel for graceful shutdown communication
    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let shutdown_signal = ShutdownSignal::new();
    let shutdown_timeout = Duration::from_secs(10); // 10 second graceful shutdown timeout

    let on_shutdown = move || {
        info!("Driver shutting down gracefully...");
        // Send shutdown signal to processor
        let _ = shutdown_tx.send(());
    };

    run_until_shutdown_graceful(
        async move { driver.start_with_shutdown(Some(shutdown_rx)).await },
        shutdown_signal,
        shutdown_timeout,
        on_shutdown,
    )
    .await
}
