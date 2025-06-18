//! Shared state for API handlers and constants

use clickhouse_lib::ClickhouseReader;
use std::time::Duration as StdDuration;

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
}

impl std::fmt::Debug for ApiState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiState").finish_non_exhaustive()
    }
}

impl ApiState {
    /// Create a new [`ApiState`].
    pub const fn new(
        client: ClickhouseReader,
        max_requests: u64,
        rate_period: StdDuration,
    ) -> Self {
        Self { client, max_requests, rate_period }
    }

    /// Maximum number of requests allowed per [`rate_period`].
    pub const fn max_requests(&self) -> u64 {
        self.max_requests
    }

    /// Time window for rate limiting.
    pub const fn rate_period(&self) -> StdDuration {
        self.rate_period
    }
}
