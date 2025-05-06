//! Taikoscope configuration
use alloy_primitives::Address;
use clap::Parser;
use url::Url;

/// CLI options for taikoscope
#[derive(Debug, Clone, Parser)]
pub struct Opts {
    /// Clickhouse URL
    #[clap(long, default_value = "http://localhost:8123")]
    pub clickhouse_url: Url,
    /// L1 RPC URL
    #[clap(long, default_value = "ws://remotesmol:48546")]
    pub l1_rpc_url: Url,
    /// L2 RPC URL
    #[clap(long, default_value = "ws://mk1-masaya-replica-0:8546")]
    pub l2_rpc_url: Url,
    /// Taiko inbox address on Masaya
    #[clap(long, default_value = "0xa7B208DE7F35E924D59C2b5f7dE3bb346E8A138C")]
    pub inbox_address: Address,
    /// Taiko preconf whitelist address on Masaya
    #[clap(long, default_value = "0x3ea351Db28A9d4833Bf6c519F52766788DE14eC1")]
    pub preconf_whitelist_address: Address,
    /// Taiko wrapper address on Masaya
    #[clap(long, default_value = "0x962C95233f04Ef08E7FaA84DBd1c5171f06f5616")]
    pub taiko_wrapper_address: Address,
    /// If set, drop & re-create all tables (local/dev only)
    #[clap(long)]
    pub reset_db: bool,
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
