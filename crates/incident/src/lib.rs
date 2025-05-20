//! Incident crate: Instatus integration and helpers.
/// Base monitor implementation
pub mod base_monitor;
/// Batch proof timeout monitor
pub mod batch_proof_monitor;
/// Instatus client
pub mod client;
/// Monitor polling and orchestration for Instatus incidents
pub mod monitor;

// Re-export monitors for easy access
pub use base_monitor::Monitor;
pub use batch_proof_monitor::BatchProofTimeoutMonitor;
pub use monitor::{InstatusL1Monitor, InstatusMonitor};
