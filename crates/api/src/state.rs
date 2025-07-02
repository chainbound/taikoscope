//! Shared state for API handlers and constants

use clickhouse_lib::ClickhouseReader;
use network::http_retry;

use std::{
    sync::{Arc, Mutex},
    time::{Duration as StdDuration, Instant},
};

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
    pub(crate) http_client: Client,
    max_requests: u64,
    rate_period: StdDuration,
    price_cache: Arc<Mutex<CachedPrice>>,
}

#[derive(Debug)]
struct CachedPrice {
    value: f64,
    updated_at: Instant,
}

impl std::fmt::Debug for ApiState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiState").finish_non_exhaustive()
    }
}

impl ApiState {
    /// Create a new [`ApiState`].
    pub fn new(client: ClickhouseReader, max_requests: u64, rate_period: StdDuration) -> Self {
        Self {
            client,
            http_client: Client::new(),
            max_requests,
            rate_period,
            price_cache: Arc::new(Mutex::new(CachedPrice {
                value: 0.0,
                updated_at: Instant::now() - StdDuration::from_secs(61),
            })),
        }
    }

    /// Maximum number of requests allowed per [`rate_period`].
    pub const fn max_requests(&self) -> u64 {
        self.max_requests
    }

    /// Time window for rate limiting.
    pub const fn rate_period(&self) -> StdDuration {
        self.rate_period
    }

    /// Get the current ETH price in USD, cached for 1 minute.
    pub async fn eth_price(&self) -> eyre::Result<f64> {
        let now = Instant::now();
        {
            let cache = self.price_cache.lock().expect("lock poisoned");
            if now.duration_since(cache.updated_at) < StdDuration::from_secs(60) {
                return Ok(cache.value);
            }
        }

        let price = fetch_eth_price(&self.http_client).await?;
        let mut cache = self.price_cache.lock().expect("lock poisoned");
        *cache = CachedPrice { value: price, updated_at: now };
        Ok(price)
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
