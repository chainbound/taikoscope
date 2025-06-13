use std::time::Duration;

use eyre::{Result, eyre};
use reqwest::{Client, Url};
use serde_json::json;
use tokio::time::timeout;
use tracing::{error, info, warn};

/// Spawn a background task monitoring the provided public RPC endpoint.
///
/// The monitor calls `eth_syncing` every 60 seconds. If the call returns
/// `false`, the endpoint is considered healthy and an info message is logged.
/// If it returns `true` or times out after five seconds, the check is retried
/// after 15 seconds. Two consecutive negative results lead to an error log.
pub fn spawn_public_rpc_monitor(url: Url) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let client = Client::new();
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            check_once(&client, &url).await;
        }
    })
}

async fn check_once(client: &Client, url: &Url) {
    let first = check_syncing(client, url).await;
    let negative = match first {
        Ok(false) => {
            info!(url = url.as_str(), "public rpc healthy");
            false
        }
        Ok(true) => {
            warn!(url = url.as_str(), "public rpc syncing");
            true
        }
        Err(e) => {
            warn!(error = %e, url = url.as_str(), "public rpc check failed");
            true
        }
    };

    if negative {
        tokio::time::sleep(Duration::from_secs(15)).await;
        match check_syncing(client, url).await {
            Ok(false) => info!(url = url.as_str(), "public rpc recovered"),
            Ok(true) => error!(url = url.as_str(), "public rpc still syncing"),
            Err(e) => error!(error = %e, url = url.as_str(), "public rpc check failed again"),
        }
    }
}

async fn check_syncing(client: &Client, url: &Url) -> Result<bool> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_syncing",
        "params": []
    });
    let resp = timeout(Duration::from_secs(5), client.post(url.clone()).json(&body).send())
        .await
        .map_err(|_| eyre!("request timed out"))??;
    let value: serde_json::Value = resp.json().await?;
    let syncing = !matches!(value.get("result"), Some(serde_json::Value::Bool(false)));
    Ok(syncing)
}
