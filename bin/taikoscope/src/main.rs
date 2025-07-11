#![allow(unused_variables)]
//! Entrypoint.

use clap::Parser;
use config::Opts;
use dotenvy::dotenv;
use nats_utils::subscribe_to_events;
use runtime::{
    health,
    shutdown::{ShutdownSignal, run_until_shutdown},
};
use std::net::SocketAddr;
use tracing::info;
use tracing_subscriber::filter::EnvFilter;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    if let Ok(custom_env_file) = std::env::var("ENV_FILE") {
        dotenvy::from_filename(custom_env_file)?;
    } else {
        // Try the default .env file, and ignore if it doesn't exist.
        dotenv().ok();
    }

    let opts = Opts::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let health_addr: SocketAddr = format!("{}:{}", opts.api.host, opts.api.port).parse()?;
    tokio::spawn(async move {
        if let Err(e) = health::serve(health_addr, ShutdownSignal::new()).await {
            tracing::error!(error = %e, "Health server failed");
        }
    });

    info!("🔭 Taikoscope processor starting...");

    let shutdown_signal = ShutdownSignal::new();
    let on_shutdown = || {
        info!("👋 Taikoscope processor shutting down...");
    };

    // Connect to NATS and subscribe to events
    let nats_client = async_nats::connect(&opts.nats_url).await?;
    subscribe_to_events(&nats_client).await?;

    // Placeholder: process events from NATS
    // while let Some(event) = event_stream.next().await {
    //     // Process and store event
    // }

    let run_driver = async { Ok(()) };

    run_until_shutdown(run_driver, shutdown_signal, on_shutdown).await
}
