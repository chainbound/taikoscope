//! Primitives for the taikoscope.
/// Block analytics helpers
pub mod block_stats;
/// Hardware cost estimates
pub mod hardware;
/// HTTP retry helpers
pub mod http_retry;
/// Simple rate limiter
pub mod rate_limiter;
/// Retry layer
pub mod retries;
/// Shutdown handling
pub mod shutdown;

#[cfg(test)]
mod shutdown_test;
