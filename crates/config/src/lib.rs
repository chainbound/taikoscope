//! Taikoscope configuration
use alloy_primitives::Address;
use clap::Parser;
use url::Url;

/// CLI options for taikoscope
#[derive(Debug, Clone, Parser)]
pub struct Opts {
    /// Clickhouse URL
    #[clap(long, env = "CLICKHOUSE_URL")]
    pub clickhouse_url: Url,
    /// Clickhouse database
    #[clap(long, env = "CLICKHOUSE_DB")]
    pub clickhouse_db: String,
    /// Clickhouse username
    #[clap(long, env = "CLICKHOUSE_USERNAME")]
    pub clickhouse_username: String,
    /// Clickhouse password
    #[clap(long, env = "CLICKHOUSE_PASSWORD")]
    pub clickhouse_password: String,
    /// L1 RPC URL
    #[clap(long, env = "L1_RPC_URL")]
    pub l1_rpc_url: Url,
    /// L2 RPC URL
    #[clap(long, env = "L2_RPC_URL")]
    pub l2_rpc_url: Url,
    /// Taiko inbox address on Masaya
    #[clap(long, env = "TAIKO_INBOX_ADDRESS")]
    pub inbox_address: Address,
    /// Taiko preconf whitelist address on Masaya
    #[clap(long, env = "TAIKO_PRECONF_WHITELIST_ADDRESS")]
    pub preconf_whitelist_address: Address,
    /// Taiko wrapper address on Masaya
    #[clap(long, env = "TAIKO_WRAPPER_ADDRESS")]
    pub taiko_wrapper_address: Address,
    /// If set, drop & re-create all tables (local/dev only)
    #[clap(long)]
    pub reset_db: bool,
    /// Instatus API key
    #[clap(long, env = "INSTATUS_API_KEY")]
    pub instatus_api_key: String,
    /// Instatus page ID
    #[clap(long, env = "INSTATUS_PAGE_ID")]
    pub instatus_page_id: String,
    /// Instatus component ID
    #[clap(long, env = "INSTATUS_COMPONENT_ID")]
    pub instatus_component_id: String,
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
