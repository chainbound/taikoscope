//! Taikoscope configuration
use clap::Parser;

/// CLI options for taikoscope
#[derive(Debug, Clone, Parser)]
pub struct Opts {
    /// Clickhouse URL
    #[clap(long, default_value = "http://localhost:8123")]
    pub clickhouse_url: String,
    /// L1 RPC URL
    #[clap(long, default_value = "wss://eth.merkle.io")]
    pub l1_rpc_url: String,
    /// L2 RPC URL
    #[clap(long, default_value = "wss://taiko.drpc.org")]
    pub l2_rpc_url: String,
    /// Taiko inbox address
    /// `TaikoL1` from <https://docs.taiko.xyz/network-reference/alethia-addresses>
    #[clap(long, default_value = "0x06a9Ab27c7e2255df1815E6CC0168d7755Feb19a")]
    pub inbox_address: String,
}

#[cfg(test)]
mod tests {
    use super::Opts;

    #[test]
    fn test_verify_cli() {
        use clap::CommandFactory;
        Opts::command().debug_assert()
    }
}
