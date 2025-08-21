//! Sequencer operator mapping loaded from the dashboard config at build time.
//! This ensures parity between backend filtering and UI naming.

// Generated at build time from dashboard/sequencerConfig.ts
include!(concat!(env!("OUT_DIR"), "/sequencer_mapping.rs"));

/// Return ClickHouse SQL array literals for transform() mapping.
/// Example: ("['0xabc', '0xdef']", "['Gattaca', 'Chainbound']")
pub fn transform_arrays_sql() -> (String, String) {
    // Addresses are already lowercased by build.rs
    let addrs = SEQUENCER_ADDRS.iter().map(|s| format!("'{}'", s)).collect::<Vec<_>>().join(", ");
    let names = SEQUENCER_NAMES
        .iter()
        .map(|s| s.replace('\'', "\\'"))
        .map(|s| format!("'{}'", s))
        .collect::<Vec<_>>()
        .join(", ");
    (format!("[{}]", addrs), format!("[{}]", names))
}
