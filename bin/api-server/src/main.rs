//! API server binary

use std::net::SocketAddr;

use api::run;
use clap::Parser;
use clickhouse::ClickhouseClient;
use config::Opts;
use dotenvy::dotenv;
use tracing_subscriber::filter::EnvFilter;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenv().ok();
    let opts = Opts::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let client = ClickhouseClient::new(
        opts.clickhouse.url,
        opts.clickhouse.db,
        opts.clickhouse.username,
        opts.clickhouse.password,
    )?;

    let addr: SocketAddr = format!("{}:{}", opts.api.host, opts.api.port).parse()?;
    run(addr, client).await
}
