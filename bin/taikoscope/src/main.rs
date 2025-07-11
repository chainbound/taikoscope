#![allow(unused_variables)]
//! Entrypoint.

use clap::Parser;
use config::Opts;
use dotenvy::dotenv;
use runtime::{
    health,
    shutdown::{ShutdownSignal, run_until_shutdown},
};
use std::net::SocketAddr;
use tokio_stream::StreamExt;
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

    let run_driver = async {
        let js = async_nats::jetstream::new(nats_client);
        let stream = js
            .get_or_create_stream(async_nats::jetstream::stream::Config {
                name: "taiko_events".to_string(),
                subjects: vec!["taiko.events".to_string()],
                ..Default::default()
            })
            .await?;
        let consumer = stream
            .get_or_create_consumer(
                "processor",
                async_nats::jetstream::consumer::pull::Config {
                    durable_name: Some("processor".to_string()),
                    ..Default::default()
                },
            )
            .await?;
        let mut messages = consumer.messages().await?;

        while let Some(msg_res) = messages.next().await {
            if let Ok(msg) = msg_res {
                let payload = String::from_utf8_lossy(&msg.payload);
                info!("Received event: {}", payload);
                let _ = msg.ack().await;
            }
        }
        Ok::<(), eyre::Error>(())
    };

    run_until_shutdown(run_driver, shutdown_signal, on_shutdown).await
}
