//! Taikoscope configuration
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::cognitive_complexity)]
use alloy_primitives::Address;
use clap::Parser;
use url::Url;

/// Default origins allowed to access the API.
pub const DEFAULT_ALLOWED_ORIGINS: &str = "https://taikoscope.xyz,https://www.taikoscope.xyz,https://masaya.taikoscope.xyz,https://www.masaya.taikoscope.xyz,https://hekla.taikoscope.xyz,https://www.hekla.taikoscope.xyz";
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

/// Nats client configuration options
#[derive(Debug, Clone, Parser)]
pub struct NatsOpts {
    /// Nats server URL
    #[clap(long = "nats-url", env = "NATS_URL")]
    pub nats_url: Option<Url>,
    /// Nats username
    #[clap(id = "nats_username", long = "nats-username", env = "NATS_USERNAME")]
    pub username: Option<String>,
    /// Nats password
    #[clap(id = "nats_password", long = "nats-password", env = "NATS_PASSWORD")]
    pub password: Option<String>,
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
    /// Public RPC URL for health checks
    #[clap(long, env = "PUBLIC_RPC")]
    pub public_url: Option<Url>,
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
    /// Instatus component ID for batch submission monitor
    #[clap(long, env = "INSTATUS_BATCH_SUBMISSION_COMPONENT_ID", default_value = "")]
    pub batch_submission_component_id: String,
    /// Instatus component ID for proof submission timeout monitor
    #[clap(long, env = "INSTATUS_PROOF_SUBMISSION_COMPONENT_ID", default_value = "")]
    pub proof_submission_component_id: String,
    /// Instatus component ID for proof verification timeout monitor
    #[clap(long, env = "INSTATUS_PROOF_VERIFICATION_COMPONENT_ID", default_value = "")]
    pub proof_verification_component_id: String,
    /// Instatus component ID for transaction sequencing monitor
    #[clap(long, env = "INSTATUS_TRANSACTION_SEQUENCING_COMPONENT_ID", default_value = "")]
    pub transaction_sequencing_component_id: String,
    /// Enable all Instatus monitors
    #[clap(long = "enable-monitors", env = "INSTATUS_MONITORS_ENABLED", default_value_t = true)]
    pub monitors_enabled: bool,
    /// Instatus monitor poll interval in seconds
    #[clap(long, env = "INSTATUS_MONITOR_POLL_INTERVAL_SECS", default_value = "30")]
    pub monitor_poll_interval_secs: u64,
    /// Instatus monitor threshold in seconds for detecting an outage
    #[clap(long, env = "INSTATUS_MONITOR_THRESHOLD_SECS", default_value = "192")]
    pub monitor_threshold_secs: u64,

    /// Batch proof timeout threshold in seconds (default 3 hours)
    #[clap(long, env = "BATCH_PROOF_TIMEOUT_SECS", default_value = "10800")]
    pub batch_proof_timeout_secs: u64,
}

impl InstatusOpts {
    /// Returns `true` if all required values are set.
    #[allow(clippy::missing_const_for_fn)]
    pub fn enabled(&self) -> bool {
        if self.api_key.is_empty() ||
            self.page_id.is_empty() ||
            self.batch_submission_component_id.is_empty() ||
            self.proof_submission_component_id.is_empty() ||
            self.proof_verification_component_id.is_empty() ||
            self.transaction_sequencing_component_id.is_empty()
        {
            return false;
        }

        true
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
    /// Allowed CORS origins (comma separated)
    #[clap(
        long = "allowed-origin",
        env = "ALLOWED_ORIGINS",
        value_delimiter = ',',
        default_value = DEFAULT_ALLOWED_ORIGINS
    )]
    pub allowed_origins: Vec<String>,

    /// Maximum number of requests allowed during the rate limiting period
    #[clap(
        long = "rate-limit-max-requests",
        env = "RATE_LIMIT_MAX_REQUESTS",
        default_value = "1000"
    )]
    pub rate_limit_max_requests: u64,

    /// Duration of the rate limiting window in seconds
    #[clap(long = "rate-limit-period-secs", env = "RATE_LIMIT_PERIOD_SECS", default_value = "60")]
    pub rate_limit_period_secs: u64,
}

/// CLI options for taikoscope
#[derive(Debug, Clone, Parser)]
pub struct Opts {
    /// Clickhouse database configuration
    #[clap(flatten)]
    pub clickhouse: ClickhouseOpts,

    /// Nats client configuration
    #[clap(flatten)]
    pub nats: NatsOpts,

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
    //! Tests that modify environment variables need to be run with --test-threads=1
    //! to avoid interference between parallel test execution.
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
            "--nats-url",
            "nats://localhost:4222",
            "--nats-username",
            "natsuser",
            "--nats-password",
            "natspass",
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
            "--batch-submission-component-id",
            "batch",
            "--proof-submission-component-id",
            "proof",
            "--proof-verification-component-id",
            "verify",
            "--transaction-sequencing-component-id",
            "l2",
            "--api-host",
            "127.0.0.1",
            "--api-port",
            "3000",
        ]
    }

    #[test]
    fn test_default_values() {
        // Clean up any environment variables that might affect this test
        use std::env;
        unsafe {
            env::remove_var("INSTATUS_MONITOR_POLL_INTERVAL_SECS");
            env::remove_var("INSTATUS_MONITOR_THRESHOLD_SECS");
            env::remove_var("BATCH_PROOF_TIMEOUT_SECS");
            env::remove_var("INSTATUS_MONITORS_ENABLED");
            env::remove_var("ALLOWED_ORIGINS");
            env::remove_var("RATE_LIMIT_MAX_REQUESTS");
            env::remove_var("RATE_LIMIT_PERIOD_SECS");
        }

        let args = base_args();
        let opts = Opts::try_parse_from(args).expect("failed to parse opts");

        assert_eq!(opts.instatus.monitor_poll_interval_secs, 30);
        assert_eq!(opts.instatus.monitor_threshold_secs, 192);
        assert_eq!(opts.instatus.batch_proof_timeout_secs, 10800);
        assert_eq!(opts.api.host, "127.0.0.1");
        assert_eq!(opts.api.port, 3000);

        let expected_origins = vec![
            "https://taikoscope.xyz",
            "https://www.taikoscope.xyz",
            "https://masaya.taikoscope.xyz",
            "https://www.masaya.taikoscope.xyz",
            "https://hekla.taikoscope.xyz",
            "https://www.hekla.taikoscope.xyz",
        ];
        assert_eq!(opts.api.allowed_origins, expected_origins);

        assert_eq!(opts.api.rate_limit_max_requests, 1000);
        assert_eq!(opts.api.rate_limit_period_secs, 60);
        assert!(!opts.reset_db);
    }

    #[test]
    fn test_env_overrides() {
        use std::env;

        // Clean up first to ensure clean state
        unsafe {
            env::remove_var("INSTATUS_MONITOR_POLL_INTERVAL_SECS");
            env::remove_var("INSTATUS_MONITOR_THRESHOLD_SECS");
            env::remove_var("BATCH_PROOF_TIMEOUT_SECS");
            env::remove_var("INSTATUS_MONITORS_ENABLED");
            env::remove_var("ALLOWED_ORIGINS");
            env::remove_var("RATE_LIMIT_MAX_REQUESTS");
            env::remove_var("RATE_LIMIT_PERIOD_SECS");
        }

        unsafe {
            env::set_var("INSTATUS_MONITOR_POLL_INTERVAL_SECS", "42");
            env::set_var("INSTATUS_MONITOR_THRESHOLD_SECS", "33");
            env::set_var("BATCH_PROOF_TIMEOUT_SECS", "99");
            env::set_var("INSTATUS_MONITORS_ENABLED", "false");
            env::set_var("ALLOWED_ORIGINS", "http://localhost:3000,http://localhost:5173");
            env::set_var("RATE_LIMIT_MAX_REQUESTS", "500");
            env::set_var("RATE_LIMIT_PERIOD_SECS", "120");
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
        assert_eq!(opts.api.rate_limit_max_requests, 500);
        assert_eq!(opts.api.rate_limit_period_secs, 120);
        assert!(opts.reset_db);

        // Clean up after test
        unsafe {
            env::remove_var("INSTATUS_MONITOR_POLL_INTERVAL_SECS");
            env::remove_var("INSTATUS_MONITOR_THRESHOLD_SECS");
            env::remove_var("BATCH_PROOF_TIMEOUT_SECS");
            env::remove_var("INSTATUS_MONITORS_ENABLED");
            env::remove_var("ALLOWED_ORIGINS");
            env::remove_var("RATE_LIMIT_MAX_REQUESTS");
            env::remove_var("RATE_LIMIT_PERIOD_SECS");
        }
    }

    #[test]
    fn test_all_origins_included() {
        use super::DEFAULT_ALLOWED_ORIGINS;

        assert!(DEFAULT_ALLOWED_ORIGINS.contains("taikoscope.xyz"));
        assert!(DEFAULT_ALLOWED_ORIGINS.contains("www.taikoscope.xyz"));
        assert!(DEFAULT_ALLOWED_ORIGINS.contains("hekla.taikoscope.xyz"));
        assert!(DEFAULT_ALLOWED_ORIGINS.contains("www.hekla.taikoscope.xyz"));

        // Verify all origins are present
        let origins: Vec<&str> = DEFAULT_ALLOWED_ORIGINS.split(',').collect();
        assert_eq!(origins.len(), 6);
        assert!(origins.contains(&"https://taikoscope.xyz"));
        assert!(origins.contains(&"https://www.taikoscope.xyz"));
        assert!(origins.contains(&"https://masaya.taikoscope.xyz"));
        assert!(origins.contains(&"https://www.masaya.taikoscope.xyz"));
        assert!(origins.contains(&"https://hekla.taikoscope.xyz"));
        assert!(origins.contains(&"https://www.hekla.taikoscope.xyz"));
    }
}
