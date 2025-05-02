//! Taikoscope configuration
use clap::Parser;

/// CLI options for taikoscope
#[derive(Debug, Clone, Parser)]
pub struct Opts {
    /// Clickhouse URL
    #[clap(long, default_value = "http://localhost:8123")]
    pub clickhouse_url: String,
    /// L1 RPC URL
    #[clap(long, default_value = "wss://remotesmol:48546")]
    pub l1_rpc_url: String,
    /// L2 RPC URL
    #[clap(long, default_value = "wss://mk1-masaya-replica-0:8546")]
    pub l2_rpc_url: String,
    /// Taiko inbox address on Masaya
    #[clap(long, default_value = "0xa7B208DE7F35E924D59C2b5f7dE3bb346E8A138C")]
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
