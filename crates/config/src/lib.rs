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
    #[clap(long, env = "INSTATUS_API_KEY", default_value = "")]
    pub api_key: String,
    /// Instatus page ID
    #[clap(long, env = "INSTATUS_PAGE_ID", default_value = "")]
    pub page_id: String,
    /// Instatus component ID for batch proposals monitor
    #[clap(long, env = "INSTATUS_BATCH_COMPONENT_ID", default_value = "")]
    pub batch_component_id: String,
    /// Instatus component ID for batch proof timeout monitor
    #[clap(long, env = "INSTATUS_BATCH_PROOF_TIMEOUT_COMPONENT_ID", default_value = "")]
    pub batch_proof_timeout_component_id: String,
    /// Instatus component ID for batch verify timeout monitor
    #[clap(long, env = "INSTATUS_BATCH_VERIFY_TIMEOUT_COMPONENT_ID", default_value = "")]
    pub batch_verify_timeout_component_id: String,
    /// Instatus component ID for L2 head monitor
    #[clap(long, env = "INSTATUS_L2_COMPONENT_ID", default_value = "")]
    pub l2_component_id: String,
    /// Instatus monitor poll interval in seconds
    #[clap(long, env = "INSTATUS_MONITOR_POLL_INTERVAL_SECS", default_value = "30")]
    pub monitor_poll_interval_secs: u64,
    /// Instatus monitor threshold in seconds for detecting an outage
    #[clap(long, env = "INSTATUS_MONITOR_THRESHOLD_SECS", default_value = "96")]
    pub monitor_threshold_secs: u64,

    /// Batch proof timeout threshold in seconds (default 3 hours)
    #[clap(long, env = "BATCH_PROOF_TIMEOUT_SECS", default_value = "10800")]
    pub batch_proof_timeout_secs: u64,
}

impl InstatusOpts {
    /// Returns `true` if all required values are set.
    #[allow(clippy::missing_const_for_fn)]
    pub fn enabled(&self) -> bool {
        !(self.api_key.is_empty() ||
            self.page_id.is_empty() ||
            self.batch_component_id.is_empty() ||
            self.batch_proof_timeout_component_id.is_empty() ||
            self.batch_verify_timeout_component_id.is_empty() ||
            self.l2_component_id.is_empty())
    }
}

/// API server configuration options
#[derive(Debug, Clone, Parser)]
pub struct ApiOpts {
    /// API server host
    #[clap(long = "api-host", env = "API_HOST", default_value = "127.0.0.1")]
    pub host: String,
    /// API server port
    #[clap(long = "api-port", env = "API_PORT", default_value = "3000")]
    pub port: u16,
    /// Additional allowed CORS origins (comma separated)
    #[clap(long = "allowed-origin", env = "ALLOWED_ORIGINS", value_delimiter = ',')]
    pub allowed_origins: Vec<String>,
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

    /// API server configuration
    #[clap(flatten)]
    pub api: ApiOpts,

    /// If set, drop & re-create all tables (local/dev only)
    #[clap(long)]
    pub reset_db: bool,
}

#[cfg(test)]
mod tests {
    use super::Opts;
    use clap::Parser;

    #[test]
    fn test_verify_cli() {
        use clap::CommandFactory;
        Opts::command().debug_assert()
    }

    fn base_args() -> Vec<&'static str> {
        vec![
            "prog",
            "--url",
            "http://localhost:8123",
            "--db",
            "test-db",
            "--username",
            "user",
            "--password",
            "pass",
            "--l1-url",
            "http://l1",
            "--l2-url",
            "http://l2",
            "--inbox-address",
            "0x0000000000000000000000000000000000000001",
            "--preconf-whitelist-address",
            "0x0000000000000000000000000000000000000002",
            "--taiko-wrapper-address",
            "0x0000000000000000000000000000000000000003",
            "--api-key",
            "key",
            "--page-id",
            "page",
            "--batch-component-id",
            "batch",
            "--batch-proof-timeout-component-id",
            "proof",
            "--batch-verify-timeout-component-id",
            "verify",
            "--l2-component-id",
            "l2",
            "--api-host",
            "127.0.0.1",
            "--api-port",
            "3000",
        ]
    }

    #[test]
    fn test_default_values() {
        let args = base_args();
        let opts = Opts::try_parse_from(args).expect("failed to parse opts");

        assert_eq!(opts.instatus.monitor_poll_interval_secs, 30);
        assert_eq!(opts.instatus.monitor_threshold_secs, 96);
        assert_eq!(opts.instatus.batch_proof_timeout_secs, 10800);
        assert_eq!(opts.api.host, "127.0.0.1");
        assert_eq!(opts.api.port, 3000);
        assert!(opts.api.allowed_origins.is_empty());
        assert!(!opts.reset_db);
    }

    #[test]
    fn test_env_overrides() {
        use std::env;

        unsafe {
            env::set_var("INSTATUS_MONITOR_POLL_INTERVAL_SECS", "42");
            env::set_var("INSTATUS_MONITOR_THRESHOLD_SECS", "33");
            env::set_var("BATCH_PROOF_TIMEOUT_SECS", "99");
            env::set_var("ALLOWED_ORIGINS", "http://localhost:3000,http://localhost:5173");
        }

        let mut args = base_args();
        args.push("--reset-db");

        let opts = Opts::try_parse_from(&args).expect("failed to parse opts");

        assert_eq!(opts.instatus.monitor_poll_interval_secs, 42);
        assert_eq!(opts.instatus.monitor_threshold_secs, 33);
        assert_eq!(opts.instatus.batch_proof_timeout_secs, 99);
        assert_eq!(opts.api.host, "127.0.0.1");
        assert_eq!(opts.api.port, 3000);
        assert_eq!(
            opts.api.allowed_origins,
            vec!["http://localhost:3000", "http://localhost:5173",]
        );
        assert!(opts.reset_db);

        unsafe {
            env::remove_var("INSTATUS_MONITOR_POLL_INTERVAL_SECS");
            env::remove_var("INSTATUS_MONITOR_THRESHOLD_SECS");
            env::remove_var("BATCH_PROOF_TIMEOUT_SECS");
            env::remove_var("ALLOWED_ORIGINS");
        }
    }
}
