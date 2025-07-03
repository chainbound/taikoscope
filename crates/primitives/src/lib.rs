//! Core primitives for the Taikoscope project.
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::cognitive_complexity)]
/// Block analytics helpers
pub mod block_stats;
/// Hardware cost estimates
pub mod hardware;
/// Block header types
pub mod headers;
/// L1 data cost calculation helpers
pub mod l1_data_cost;

/// Number of wei in one gwei.
pub const WEI_PER_GWEI: u128 = 1_000_000_000;
