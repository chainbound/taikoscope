//! Incident crate: Instatus integration and helpers.
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::cognitive_complexity)]
/// Base monitor implementation
pub mod base_monitor;
/// Instatus client
pub mod client;
/// Monitor polling and orchestration for Instatus incidents
pub mod monitor;
/// Retry helpers for HTTP operations
pub mod retry;
/// Shared helpers for payload building and retrying operations
pub mod helpers;

// Re-export monitors for easy access
pub use base_monitor::Monitor;
pub use monitor::{BatchProofTimeoutMonitor, InstatusL1Monitor, InstatusMonitor};
