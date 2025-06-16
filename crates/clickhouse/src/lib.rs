//! `ClickHouse` database integration for Taikoscope
//!
//! This crate provides two main components:
//! - `ClickhouseWriter`: For taikoscope data insertion and database management
//! - `ClickhouseReader`: For API read-only operations and analytics
//!
//! The separation allows taikoscope to control data, migrations, and inserts,
//! while the API can only read data for serving analytics and dashboards.

pub use primitives::headers::{L1Header, L2Header};

// Re-export core functionality
/// Type conversions between external types and internal models
pub mod conversions;
/// Data models and structures for `ClickHouse` tables
pub mod models;
/// Read-only client for API operations
pub mod reader;
/// Schema definitions and table structures
pub mod schema;
/// Byte wrapper types used throughout the crate
pub mod types;
/// Write operations client for taikoscope
pub mod writer;

// Re-export main types for convenience
pub use reader::{ClickhouseReader, TimeRange};
pub use writer::ClickhouseWriter;

// Re-export all models for backward compatibility and ease of use
pub use models::*;

// Re-export schema constants
pub use schema::{TABLE_SCHEMAS, TABLES, VIEWS};

// Re-export byte wrappers
pub use types::{AddressBytes, HashBytes};

// Re-export test utilities for testing across the workspace
#[cfg(feature = "test-util")]
pub use clickhouse::test;

/// Legacy alias for backward compatibility - will be deprecated
pub type ClickhouseClient = ClickhouseWriter;
