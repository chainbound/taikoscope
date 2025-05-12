//! Incident crate: Instatus integration and helpers.
/// Instatus client
pub mod client;
/// Monitor polling and orchestration for Instatus incidents
pub mod monitor;

// Re-export monitor for easy access
pub use monitor::InstatusMonitor;
