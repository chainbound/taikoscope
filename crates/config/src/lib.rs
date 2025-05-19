//! Taikoscope configuration
use alloy_primitives::Address;
use clap::Parser;
use url::Url;

/// Clickhouse database configuration options
#[derive(Debug, Clone, Parser)]
pub struct ClickhouseOpts {
    /// Clickhouse URL
    #[clap(long, env = "CLICKHOUSE_URL")]
    pub url: Url,
    /// Clickhouse database
    #[clap(long, env = "CLICKHOUSE_DB")]
    pub db: String,
    /// Clickhouse username
    #[clap(long, env = "CLICKHOUSE_USERNAME")]
    pub username: String,
    /// Clickhouse password
    #[clap(long, env = "CLICKHOUSE_PASSWORD")]
    pub password: String,
}

/// RPC endpoint configuration options
#[derive(Debug, Clone, Parser)]
pub struct RpcOpts {
    /// L1 RPC URL
    #[clap(long, env = "L1_RPC_URL")]
    pub l1_url: Url,
    /// L2 RPC URL
    #[clap(long, env = "L2_RPC_URL")]
    pub l2_url: Url,
}

/// Taiko contract address configuration options
#[derive(Debug, Clone, Parser)]
pub struct TaikoAddressOpts {
    /// Taiko inbox address on Masaya
    #[clap(long, env = "TAIKO_INBOX_ADDRESS")]
    pub inbox_address: Address,
    /// Taiko preconf whitelist address on Masaya
    #[clap(long, env = "TAIKO_PRECONF_WHITELIST_ADDRESS")]
    pub preconf_whitelist_address: Address,
    /// Taiko wrapper address on Masaya
    #[clap(long, env = "TAIKO_WRAPPER_ADDRESS")]
    pub taiko_wrapper_address: Address,
}

/// Instatus monitoring configuration options
#[derive(Debug, Clone, Parser)]
pub struct InstatusOpts {
    /// Instatus API key
    #[clap(long, env = "INSTATUS_API_KEY")]
    pub api_key: String,
    /// Instatus page ID
    #[clap(long, env = "INSTATUS_PAGE_ID")]
    pub page_id: String,
    /// Instatus component ID for batch proposals monitor
    #[clap(long, env = "INSTATUS_BATCH_COMPONENT_ID")]
    pub batch_component_id: String,
    /// Instatus component ID for L2 head monitor
    #[clap(long, env = "INSTATUS_L2_COMPONENT_ID")]
    pub l2_component_id: String,
    /// Instatus monitor poll interval in seconds
    #[clap(long, env = "INSTATUS_MONITOR_POLL_INTERVAL_SECS", default_value = "30")]
    pub monitor_poll_interval_secs: u64,
    /// Instatus monitor threshold in seconds for detecting an outage
    #[clap(long, env = "INSTATUS_MONITOR_THRESHOLD_SECS", default_value = "96")]
    pub monitor_threshold_secs: u64,
    /// Instatus monitor healthy needed count to resolve an incident
    #[clap(long, env = "INSTATUS_MONITOR_HEALTHY_NEEDED_COUNT", default_value = "2")]
    pub monitor_healthy_needed_count: u8,
}

/// CLI options for taikoscope
#[derive(Debug, Clone, Parser)]
pub struct Opts {
    /// Clickhouse database configuration
    #[clap(flatten)]
    pub clickhouse: ClickhouseOpts,

    /// RPC endpoint configuration
    #[clap(flatten)]
    pub rpc: RpcOpts,

    /// Taiko contract address configuration
    #[clap(flatten)]
    pub taiko_addresses: TaikoAddressOpts,

    /// Instatus monitoring configuration
    #[clap(flatten)]
    pub instatus: InstatusOpts,

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
