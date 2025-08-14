#![allow(missing_docs)]

use std::time::Duration;

use clap::Parser;
use config::{OperationMode, Opts};
use dotenvy::dotenv;
use driver::{ingestor::IngestorDriver, processor::ProcessorDriver, unified::UnifiedDriver};
use runtime::shutdown::{ShutdownSignal, run_until_shutdown, run_until_shutdown_graceful};
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

    match opts.mode {
        OperationMode::Ingestor => run_ingestor_only(opts).await,
        OperationMode::Processor => run_processor_only(opts).await,
        OperationMode::Unified => run_unified(opts).await,
    }
}

async fn run_ingestor_only(opts: Opts) -> eyre::Result<()> {
    info!("Starting Taikoscope Ingestor");

    let driver = IngestorDriver::new(opts).await?;

    let shutdown_signal = ShutdownSignal::new();
    let on_shutdown = || {
        info!("Ingestor shutting down...");
    };

    run_until_shutdown(async move { driver.start().await }, shutdown_signal, on_shutdown).await
}

async fn run_processor_only(opts: Opts) -> eyre::Result<()> {
    info!("Starting Taikoscope Processor");

    let driver = ProcessorDriver::new(opts).await?;

    // Create broadcast channel for graceful shutdown communication
    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let shutdown_signal = ShutdownSignal::new();
    let shutdown_timeout = Duration::from_secs(10); // 10 second graceful shutdown timeout

    let on_shutdown = move || {
        info!("Processor shutting down gracefully...");
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

async fn run_unified(opts: Opts) -> eyre::Result<()> {
    info!("Starting Taikoscope in Unified Mode");

    let driver = UnifiedDriver::new(opts).await?;

    // Create broadcast channel for graceful shutdown communication
    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let shutdown_signal = ShutdownSignal::new();
    let shutdown_timeout = Duration::from_secs(10); // 10 second graceful shutdown timeout

    let on_shutdown = move || {
        info!("Unified driver shutting down gracefully...");
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
