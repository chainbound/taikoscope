//! Estimated hardware cost constants for running a redundant sequencer setup.

/// Rough estimate for the cost of a single sequencer server in USD.
pub const SEQUENCER_SERVER_COST_USD: f64 = 3500.0;

/// Number of servers required for a redundant setup.
pub const SEQUENCER_SERVER_COUNT: usize = 2;

/// Estimated cost for networking and other supporting equipment in USD.
pub const NETWORKING_COST_USD: f64 = 500.0;

/// Estimated total hardware cost for a redundant sequencer setup in USD.
pub const TOTAL_HARDWARE_COST_USD: f64 =
    SEQUENCER_SERVER_COST_USD * SEQUENCER_SERVER_COUNT as f64 + NETWORKING_COST_USD;
