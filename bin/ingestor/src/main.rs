#![allow(missing_docs)]
#![allow(unused_imports)]

use clap::Parser;
use config::Opts;
use dotenvy::dotenv;
use nats_utils::{TaikoEvent, publish_event};
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

    info!("Ingestor starting...");

    // Connect to NATS
    let _nats_client = async_nats::connect(&opts.nats_url).await?;

    // Placeholder: extract events and publish to NATS
    // for event in extract_events().await {
    //     publish_event(&nats_client, &event).await?;
    // }

    Ok(())
}
