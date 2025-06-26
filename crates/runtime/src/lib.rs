//! Runtime utilities for Taikoscope.
#![allow(missing_docs)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::cognitive_complexity)]

pub mod health;
pub mod rate_limiter;
pub mod shutdown;

#[cfg(test)]
mod shutdown_test;
