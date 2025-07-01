//! Unit conversion helpers

/// Convert wei to ETH with 4 decimal precision.
pub fn wei_to_eth(value: u128) -> f64 {
    let eth = value as f64 / 1e18_f64;
    (eth * 1e4_f64).round() / 1e4_f64
}

/// Convert signed wei to ETH with 4 decimal precision.
pub fn wei_to_eth_signed(value: i128) -> f64 {
    let eth = value as f64 / 1e18_f64;
    (eth * 1e4_f64).round() / 1e4_f64
}

/// Convert optional wei to ETH.
pub fn opt_wei_to_eth(value: Option<u128>) -> Option<f64> {
    value.map(wei_to_eth)
}

/// Convert optional signed wei to ETH.
pub fn opt_wei_to_eth_signed(value: Option<i128>) -> Option<f64> {
    value.map(wei_to_eth_signed)
}
