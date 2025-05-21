//! Entrypoint.

use driver::Driver;

use clap::Parser;
use config::Opts;
use dotenvy::dotenv;
use primitives::shutdown::{ShutdownSignal, run_until_shutdown};
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

    info!("🔭 Taikoscope engine starting...");

    let shutdown_signal = ShutdownSignal::new();
    let on_shutdown = || {
        info!("👋 Taikoscope engine shutting down...");
    };

    let run_driver = async { Driver::new(opts).await?.start().await };

    run_until_shutdown(run_driver, shutdown_signal, on_shutdown).await
}
