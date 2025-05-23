//! Primitives for the taikoscope.
/// Hardware cost estimates
pub mod hardware;
/// Simple rate limiter
pub mod rate_limiter;
/// Retry layer
pub mod retries;
/// Shutdown handling
pub mod shutdown;

#[cfg(test)]
mod shutdown_test;
