//! Shared state for API handlers and constants

use clickhouse_lib::ClickhouseReader;
use network::http_retry;

use std::{
    sync::{Arc, Mutex},
    time::Duration as StdDuration,
};

use tokio_util::sync::CancellationToken;

use reqwest::Client;
use serde_json::Value;

/// Default maximum number of requests allowed during the rate limiting period.
pub const DEFAULT_MAX_REQUESTS: u64 = u64::MAX;
/// Default duration for the rate limiting window.
pub const DEFAULT_RATE_PERIOD: StdDuration = StdDuration::from_secs(1);
/// Maximum number of records returned by the `/block-transactions` endpoint.
pub const MAX_BLOCK_TRANSACTIONS_LIMIT: u64 = 50000;
/// Maximum number of records returned by table endpoints.
pub const MAX_TABLE_LIMIT: u64 = 50000;

/// Shared state for API handlers.
#[derive(Clone)]
pub struct ApiState {
    pub(crate) client: ClickhouseReader,
    max_requests: u64,
    rate_period: StdDuration,
    price_cache: Arc<Mutex<CachedPrice>>,
    shutdown: Arc<CancellationToken>,
}

#[derive(Debug)]
struct CachedPrice {
    value: Option<f64>,
}

impl std::fmt::Debug for ApiState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiState").finish_non_exhaustive()
    }
}

impl ApiState {
    /// Create a new [`ApiState`].
    pub fn new(client: ClickhouseReader, max_requests: u64, rate_period: StdDuration) -> Self {
        let cache = Arc::new(Mutex::new(CachedPrice { value: None }));
        let token = Arc::new(CancellationToken::new());
        spawn_price_refresh(Client::new(), Arc::clone(&cache), Arc::clone(&token));
        Self { client, max_requests, rate_period, price_cache: cache, shutdown: token }
    }

    /// Maximum number of requests allowed per [`rate_period`].
    pub const fn max_requests(&self) -> u64 {
        self.max_requests
    }

    /// Time window for rate limiting.
    pub const fn rate_period(&self) -> StdDuration {
        self.rate_period
    }

    /// Get the current ETH price in USD from the cache.
    pub async fn cached_eth_price(&self) -> Option<f64> {
        let cache = self.price_cache.lock().expect("lock poisoned");
        cache.value
    }
}

async fn fetch_eth_price(client: &Client) -> eyre::Result<f64> {
    let url = std::env::var("ETH_PRICE_URL").unwrap_or_else(|_| {
        "https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd".to_owned()
    });
    let price = http_retry::retry_op(|| async {
        let resp = client.get(&url).send().await?;
        let resp = resp.error_for_status()?;
        let json: Value = resp.json().await?;
        json.get("ethereum")
            .and_then(|e| e.get("usd"))
            .and_then(|v| v.as_f64())
            .ok_or_else(|| eyre::eyre!("invalid response"))
    })
    .await?;
    Ok(price)
}

fn spawn_price_refresh(
    client: Client,
    cache: Arc<Mutex<CachedPrice>>,
    token: Arc<CancellationToken>,
) {
    tokio::spawn(async move {
        loop {
            match fetch_eth_price(&client).await {
                Ok(price) => {
                    let mut lock = cache.lock().expect("lock poisoned");
                    *lock = CachedPrice { value: Some(price) };
                }
                Err(e) => tracing::warn!(error = %e, "failed to refresh ETH price"),
            }

            tokio::select! {
                _ = token.cancelled() => break,
                _ = tokio::time::sleep(StdDuration::from_secs(60)) => {},
            }
        }
    });
}

impl Drop for ApiState {
    fn drop(&mut self) {
        if Arc::strong_count(&self.shutdown) == 1 {
            self.shutdown.cancel();
        }
    }
}
