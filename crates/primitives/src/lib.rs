//! Primitives for the taikoscope.
/// Retry layer
pub mod retries;
/// Shutdown handling
pub mod shutdown;

#[cfg(test)]
mod shutdown_test;
