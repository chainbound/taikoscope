use crate::{
    base_monitor::{BaseMonitor, Monitor},
    client::Client as IncidentClient,
};
use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use clickhouse::ClickhouseReader;
use eyre::Result;
use reqwest;
use serde::Serialize;
use std::time::Duration;
use tracing::{debug, error, info, warn};
use url::Url;

/// Incident‐level state sent to Instatus.
#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IncidentState {
    /// Incident is being investigated.
    Investigating,
    /// Incident is identified.
    Identified,
    /// Incident is being monitored.
    Monitoring,
    /// Incident is resolved.
    Resolved,
}

/// Component health inside an incident update.
#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ComponentHealth {
    /// Component is operational.
    Operational,
    /// Component is experiencing a partial outage.
    PartialOutage,
    /// Component is experiencing a major outage.
    MajorOutage,
}

/// Status for a single component.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ComponentStatus {
    /// Component ID
    pub id: String,
    /// Status (e.g. MAJOROUTAGE, OPERATIONAL)
    pub status: ComponentHealth,
}

impl ComponentStatus {
    /// Create a new component status for a major outage.
    pub fn major_outage(id: &str) -> Self {
        Self { id: id.into(), status: ComponentHealth::MajorOutage }
    }

    /// Create a new component status for an operational component.
    pub fn operational(id: &str) -> Self {
        Self { id: id.into(), status: ComponentHealth::Operational }
    }
}

/// Payload for creating a new incident.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct NewIncident {
    /// Incident name
    pub name: String,
    /// Incident message/description
    pub message: String,
    /// Incident status (e.g. INVESTIGATING)
    pub status: IncidentState,
    /// Affected component IDs
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<String>,
    /// Component statuses
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub statuses: Vec<ComponentStatus>,
    /// Whether to notify subscribers
    pub notify: bool,
    /// Optional start timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started: Option<String>,
}

/// Payload for resolving an incident.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ResolveIncident {
    /// Status (should be RESOLVED)
    pub status: IncidentState,
    /// Affected component IDs
    pub components: Vec<String>,
    /// Component statuses
    pub statuses: Vec<ComponentStatus>,
    /// Whether to notify subscribers
    pub notify: bool,
    /// Incident start time in RFC3339 format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started: Option<String>,
}

/// Monitors `ClickHouse` `BatchProposed` events and manages Instatus incidents.
/// Polls `ClickHouse` every `interval` seconds; if no batch event for `threshold` seconds
/// and a recent L2 head event within `threshold` seconds is available, it creates an incident;
/// resolves when batch events resume.
#[derive(Debug)]
pub struct InstatusL1Monitor {
    base: BaseMonitor<()>,
    threshold: Duration,
}

impl InstatusL1Monitor {
    /// Creates a new `InstatusL1Monitor` with the given parameters.
    pub fn new(
        clickhouse: ClickhouseReader,
        client: IncidentClient,
        component_id: String,
        threshold: Duration,
        interval: Duration,
    ) -> Self {
        Self { base: BaseMonitor::new(clickhouse, client, component_id, interval), threshold }
    }

    /// Handle the status of batch events
    async fn handle(&mut self, last_batch: DateTime<Utc>, last_l2: DateTime<Utc>) -> Result<()> {
        let age_batch = Utc::now().signed_duration_since(last_batch).to_std()?;
        let age_l2 = Utc::now().signed_duration_since(last_l2).to_std()?;
        let batch_healthy = !age_batch.gt(&self.threshold);
        let l2_healthy = !age_l2.gt(&self.threshold);

        debug!(
            active_incident = ?self.base.active_incidents,
            batch_age_seconds = age_batch.as_secs(),
            l2_age_seconds = age_l2.as_secs(),
            threshold_seconds = self.threshold.as_secs(),
            batch_healthy,
            l2_healthy,
            "Batch event status"
        );

        // Add startup grace period check - don't create incidents if data is too old (system just
        // started)
        let startup_grace_period = Duration::from_secs(3600); // 1 hour grace period
        let batch_very_old = age_batch > startup_grace_period;
        let l2_very_old = age_l2 > startup_grace_period;

        if batch_very_old || l2_very_old {
            debug!(
                batch_very_old,
                l2_very_old, "Skipping incident creation due to startup grace period"
            );
            return Ok(());
        }

        let has_active = !self.base.active_incidents.is_empty();

        match (has_active, batch_healthy, l2_healthy) {
            // Batch outage while L2 healthy: open incident
            (false, false, true) => {
                let id = self.open(last_batch).await?;
                self.base.active_incidents.insert((), id);
            }
            // still down
            (true, false, _) => self.base.mark_unhealthy(),
            // up again: close when stable
            (true, true, _) => {
                if self.base.mark_healthy(&()).await? {
                    // Incident resolved
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Opens a new incident
    async fn open(&self, last_batch: DateTime<Utc>) -> Result<String> {
        let _started = (last_batch + ChronoDuration::seconds(2)).to_rfc3339();
        let body = self.base.create_incident_payload(
            "No BatchProposed events - Possible Outage".into(),
            format!("No batch event for {}s", self.threshold.as_secs()),
            last_batch + ChronoDuration::seconds(2),
        );

        self.base.create_incident_with_payload(&body).await
    }

    /// Check batch and L2 head times
    async fn check_batch_and_l2(&mut self) -> Result<()> {
        let batch_res = self.base.clickhouse.get_last_batch_time().await;
        let l2_res = self.base.clickhouse.get_last_l2_head_time().await;

        match (batch_res, l2_res) {
            (Ok(Some(batch_ts)), Ok(Some(l2_ts))) => {
                // Additional validation: ensure we have sufficient data before monitoring
                let now = Utc::now();
                let batch_age = now.signed_duration_since(batch_ts).to_std()?;
                let l2_age = now.signed_duration_since(l2_ts).to_std()?;

                // Skip monitoring if timestamps are suspiciously old (indicates incomplete data)
                let max_reasonable_age = Duration::from_secs(86400); // 24 hours
                if batch_age > max_reasonable_age || l2_age > max_reasonable_age {
                    debug!(
                        batch_age_hours = batch_age.as_secs() / 3600,
                        l2_age_hours = l2_age.as_secs() / 3600,
                        "Skipping monitoring due to suspiciously old timestamps - insufficient data"
                    );
                    return Ok(());
                }

                if let Err(e) = self.handle(batch_ts, l2_ts).await {
                    error!(%e, "handling new batch event status");
                }
            }
            (Ok(None), Ok(Some(_))) => {
                debug!("no batch event timestamp available this tick for batch monitor - skipping")
            }
            (_, Ok(None)) => {
                debug!("no L2 head timestamp available this tick for batch monitor - skipping")
            }
            (Err(e), _) => error!(%e, "failed to query last batch time"),
            (_, Err(e)) => error!(%e, "failed to query last L2 head time for batch monitor"),
        }

        Ok(())
    }

    /// Check for existing incidents and initial health
    async fn check_initial_health(&mut self) -> Result<()> {
        if let Some(_id) = self.base.active_incidents.values().next() {
            if let (Ok(Some(batch_ts)), Ok(Some(l2_ts))) = (
                self.base.clickhouse.get_last_batch_time().await,
                self.base.clickhouse.get_last_l2_head_time().await,
            ) {
                if let Err(e) = self.handle(batch_ts, l2_ts).await {
                    error!(%e, "Failed initial health check for existing batch incident");
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Monitor for InstatusL1Monitor {
    type IncidentKey = ();

    async fn create_incident(&self, _key: &Self::IncidentKey) -> Result<String> {
        self.open(Utc::now()).await
    }

    async fn resolve_incident(&self, incident_id: &str) -> Result<()> {
        let payload = self.base.create_resolve_payload();
        self.base.resolve_incident_with_payload(incident_id, &payload).await
    }

    async fn check_health(&mut self) -> Result<()> {
        self.check_batch_and_l2().await
    }

    async fn initialize(&mut self) -> Result<()> {
        self.base.check_existing_incidents(()).await?;
        self.check_initial_health().await
    }

    async fn run(mut self) -> Result<()> {
        self.initialize().await?;
        let interval_duration = self.get_interval();
        let mut interval = tokio::time::interval(interval_duration);
        loop {
            interval.tick().await;
            if let Err(e) = self.check_health().await {
                error!(error = %e, "monitoring check failed for InstatusL1Monitor");
            }
        }
    }

    fn get_interval(&self) -> Duration {
        self.base.interval
    }

    fn get_component_id(&self) -> &str {
        &self.base.component_id
    }

    fn get_client(&self) -> &IncidentClient {
        &self.base.client
    }

    fn get_clickhouse(&self) -> &ClickhouseReader {
        &self.base.clickhouse
    }
}

/// Monitors the Hekla RPC endpoint via `eth_syncing`.
#[derive(Debug)]
pub struct HeklaRpcMonitor {
    base: BaseMonitor<()>,
    rpc_url: Url,
    client: reqwest::Client,
}

impl HeklaRpcMonitor {
    /// Creates a new `HeklaRpcMonitor`.
    pub fn new(
        clickhouse: ClickhouseReader,
        client: IncidentClient,
        component_id: String,
        rpc_url: Url,
        interval: Duration,
    ) -> Self {
        Self {
            base: BaseMonitor::new(clickhouse, client, component_id, interval),
            rpc_url,
            client: reqwest::Client::new(),
        }
    }

    async fn open(&self) -> Result<String> {
        if let Some(id) = self.base.client.open_incident(&self.base.component_id).await? {
            tracing::info!(incident_id = %id, "existing incident found, skipping creation");
            return Ok(id);
        }

        let body = self.base.create_incident_payload(
            "Hekla RPC not in sync".into(),
            "Hekla RPC not in sync".into(),
            Utc::now(),
        );

        self.base.create_incident_with_payload(&body).await
    }

    async fn check_rpc(&mut self) -> Result<()> {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_syncing",
            "params": [],
            "id": 1
        });

        let res: eyre::Result<serde_json::Value> = network::http_retry::retry_op(|| async {
            let resp = self.client.post(self.rpc_url.clone()).json(&payload).send().await?;
            let resp = resp.error_for_status()?;
            Ok(resp.json::<serde_json::Value>().await?)
        })
        .await;

        let healthy = matches!(
            res.as_ref().ok().and_then(|v| v.get("result")),
            Some(serde_json::Value::Bool(false))
        );

        if healthy {
            self.base.mark_healthy(&()).await?;
        } else {
            self.base.mark_unhealthy();
            if self.base.active_incidents.is_empty() {
                let id = self.open().await?;
                self.base.active_incidents.insert((), id);
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Monitor for HeklaRpcMonitor {
    type IncidentKey = ();

    async fn create_incident(&self, _key: &Self::IncidentKey) -> Result<String> {
        self.open().await
    }

    async fn resolve_incident(&self, incident_id: &str) -> Result<()> {
        let payload = self.base.create_resolve_payload();
        self.base.resolve_incident_with_payload(incident_id, &payload).await
    }

    async fn check_health(&mut self) -> Result<()> {
        self.check_rpc().await
    }

    async fn initialize(&mut self) -> Result<()> {
        self.base.check_existing_incidents(()).await
    }

    async fn run(mut self) -> Result<()> {
        self.initialize().await?;
        let mut interval = tokio::time::interval(self.get_interval());
        loop {
            interval.tick().await;
            if let Err(e) = self.check_health().await {
                error!(error = %e, "monitoring check failed for HeklaRpcMonitor");
            }
        }
    }

    fn get_interval(&self) -> Duration {
        self.base.interval
    }

    fn get_component_id(&self) -> &str {
        &self.base.component_id
    }

    fn get_client(&self) -> &IncidentClient {
        &self.base.client
    }

    fn get_clickhouse(&self) -> &ClickhouseReader {
        &self.base.clickhouse
    }
}

/// Monitors `ClickHouse` L2 head events and manages Instatus incidents.
/// Polls `ClickHouse` every `interval` seconds; if no L2 head event for `threshold` seconds, it
/// creates an incident; resolves when events resume.
#[derive(Debug)]
pub struct InstatusMonitor {
    base: BaseMonitor<()>,
    threshold: Duration,
}

impl InstatusMonitor {
    /// Creates a new `InstatusMonitor` with the given parameters.
    pub fn new(
        clickhouse: ClickhouseReader,
        client: IncidentClient,
        component_id: String,
        threshold: Duration,
        interval: Duration,
    ) -> Self {
        Self { base: BaseMonitor::new(clickhouse, client, component_id, interval), threshold }
    }

    /// Handles a new L2 head event.
    async fn handle(&mut self, last: DateTime<Utc>) -> Result<()> {
        let age = Utc::now().signed_duration_since(last).to_std()?;
        let is_healthy = !age.gt(&self.threshold);

        debug!(
            active_incident = ?self.base.active_incidents,
            age_seconds = ?age.as_secs(),
            threshold_seconds = ?self.threshold.as_secs(),
            is_healthy = is_healthy,
            "L2 head event status"
        );

        let has_active = !self.base.active_incidents.is_empty();

        match (has_active, is_healthy) {
            // outage begins
            (false, false) => {
                let id = self.open(last).await?;
                self.base.active_incidents.insert((), id);
            }
            // still down
            (true, false) => self.base.mark_unhealthy(),
            // up again
            (true, true) => {
                if self.base.mark_healthy(&()).await? {
                    // Incident resolved
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Opens a new incident.
    async fn open(&self, last: DateTime<Utc>) -> Result<String> {
        // Check if an incident already exists to avoid duplicates
        if let Some(id) = self.base.client.open_incident(&self.base.component_id).await? {
            tracing::info!(incident_id = %id, "existing incident found, skipping creation");
            return Ok(id);
        }

        // The incident starts when the L2 block should have been processed
        let _started = (last + ChronoDuration::seconds(2)).to_rfc3339();

        let body = self.base.create_incident_payload(
            "No L2 head events - Possible Outage".into(),
            format!("No L2 head event for {}s", self.threshold.as_secs()),
            last + ChronoDuration::seconds(2),
        );

        self.base.create_incident_with_payload(&body).await
    }

    /// Check for L2 head events
    async fn check_l2_head(&mut self) -> Result<()> {
        match self.base.clickhouse.get_last_l2_head_time().await {
            Ok(Some(ts)) => {
                if let Err(e) = self.handle(ts).await {
                    error!(%e, "handling new L2 head");
                }
            }
            Ok(None) => {
                warn!("no L2 head timestamp available this tick");
            }
            Err(e) => {
                error!(%e, "failed to query last L2 head time");
            }
        }

        Ok(())
    }

    /// Check for existing incidents and initial health
    async fn check_initial_health(&mut self) -> Result<()> {
        if self.base.active_incidents.values().next().is_some() {
            // Immediately check if the incident should be closed by checking latest L2 head time
            if let Ok(Some(ts)) = self.base.clickhouse.get_last_l2_head_time().await {
                info!(
                    last_l2_timestamp = %ts,
                    "Found L2 head event on startup, checking if incident can be closed"
                );
                if let Err(e) = self.handle(ts).await {
                    error!(%e, "Failed initial health check for existing incident");
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Monitor for InstatusMonitor {
    type IncidentKey = ();

    async fn create_incident(&self, _key: &Self::IncidentKey) -> Result<String> {
        self.open(Utc::now()).await
    }

    async fn resolve_incident(&self, incident_id: &str) -> Result<()> {
        let payload = self.base.create_resolve_payload();
        self.base.resolve_incident_with_payload(incident_id, &payload).await
    }

    async fn check_health(&mut self) -> Result<()> {
        self.check_l2_head().await
    }

    async fn initialize(&mut self) -> Result<()> {
        self.base.check_existing_incidents(()).await?;
        self.check_initial_health().await
    }

    async fn run(mut self) -> Result<()> {
        self.initialize().await?;
        let interval_duration = self.get_interval();
        let mut interval = tokio::time::interval(interval_duration);
        loop {
            interval.tick().await;
            if let Err(e) = self.check_health().await {
                error!(error = %e, "monitoring check failed for InstatusMonitor");
            }
        }
    }

    fn get_interval(&self) -> Duration {
        self.base.interval
    }

    fn get_component_id(&self) -> &str {
        &self.base.component_id
    }

    fn get_client(&self) -> &IncidentClient {
        &self.base.client
    }

    fn get_clickhouse(&self) -> &ClickhouseReader {
        &self.base.clickhouse
    }
}

/// Monitors batches that take too long to prove (> 3 hours after being posted).
/// Creates incidents for batches that have been posted but not proven within the time threshold.
#[derive(Debug)]
pub struct BatchProofTimeoutMonitor {
    /// Base monitor implementation
    base: BaseMonitor<(u64, u64)>,
    /// Timeout threshold for batch proofs
    proof_timeout: Duration,
}

impl BatchProofTimeoutMonitor {
    /// Creates a new `BatchProofTimeoutMonitor` with the given parameters.
    pub fn new(
        clickhouse: ClickhouseReader,
        client: IncidentClient,
        component_id: String,
        proof_timeout: Duration,
        interval: Duration,
    ) -> Self {
        Self { base: BaseMonitor::new(clickhouse, client, component_id, interval), proof_timeout }
    }

    /// Check if a specific batch has been proven
    async fn is_batch_proven(&self, batch_id: u64) -> Result<bool> {
        let proved_batch_ids = self.get_proved_batch_ids().await?;
        Ok(proved_batch_ids.contains(&batch_id))
    }

    /// Get all batch IDs that have been proven
    async fn get_proved_batch_ids(&self) -> Result<Vec<u64>> {
        self.base.clickhouse.get_proved_batch_ids().await
    }

    /// Creates an incident for an unproven batch
    async fn open_incident(
        &self,
        batch_id: u64,
        posted_at: DateTime<Utc>,
        age_hours: u64,
    ) -> Result<String> {
        // Compute incident start time preserving full precision
        let incident_start_time = posted_at + ChronoDuration::from_std(self.proof_timeout)?;
        let _started = incident_start_time.to_rfc3339();

        let body = self.base.create_incident_payload(
            format!("Batch #{} Not Proven - Timeout", batch_id),
            format!(
                "Batch #{} has been waiting for proof for {}h (threshold: {}h)",
                batch_id,
                age_hours,
                self.proof_timeout.as_secs() / 3600
            ),
            incident_start_time,
        );

        let id = self.base.create_incident_with_payload(&body).await?;

        info!(
            incident_id = %id,
            batch_id = batch_id,
            "Created batch proof timeout incident"
        );

        Ok(id)
    }

    fn filter_new_batches(
        &self,
        batches: &[(u64, u64, DateTime<Utc>)],
    ) -> Vec<(u64, u64, DateTime<Utc>)> {
        batches
            .iter()
            .filter(|(l1, batch, _)| !self.base.active_incidents.contains_key(&(*l1, *batch)))
            .copied()
            .collect()
    }

    /// Returns `true` if the only active incident is the catch-all entry.
    fn catch_all_only(&self) -> bool {
        self.base.active_incidents.len() == 1 && self.base.active_incidents.contains_key(&(0, 0))
    }

    /// Check for batches that have not been proven within the timeout period
    async fn check_unproven_batches(&mut self) -> Result<()> {
        let cutoff_time = Utc::now() - ChronoDuration::from_std(self.proof_timeout)?;
        let unproven_batches =
            self.base.clickhouse.get_unproved_batches_older_than(cutoff_time).await?;

        debug!(
            "Found {} unproven batches older than {:?}",
            unproven_batches.len(),
            self.proof_timeout
        );

        // Create incidents for new unproven batches
        for (l1_block_number, batch_id, posted_at) in self.filter_new_batches(&unproven_batches) {
            let key = (l1_block_number, batch_id);
            let age_hours = Utc::now().signed_duration_since(posted_at).num_hours();
            debug!(
                batch_id = batch_id,
                posted_at = %posted_at,
                age_hours = age_hours,
                "Found unproven batch exceeding timeout",
            );
            let incident_id = self.open_incident(batch_id, posted_at, age_hours as u64).await?;
            self.base.active_incidents.insert(key, incident_id);
        }

        // Take a snapshot of active incidents to avoid concurrent immutable/mutable borrows
        let active_incidents_snapshot: Vec<((u64, u64), String)> =
            self.base.active_incidents.iter().map(|(k, id)| (*k, id.clone())).collect();

        for (key, incident_id) in active_incidents_snapshot {
            let (_, batch_id) = key;
            if batch_id == 0 {
                continue;
            }
            let is_proven = self.is_batch_proven(batch_id).await?;
            if is_proven {
                debug!(
                    batch_id = ?batch_id,
                    incident_id = %incident_id,
                    "Batch is now proven, resolving incident immediately"
                );
                let payload = self.base.create_resolve_payload();
                self.base.resolve_incident_with_payload(&incident_id, &payload).await?;
                self.base.active_incidents.remove(&key);
            } else {
                self.base.mark_unhealthy();
            }
        }

        // Special case for the catch-all incident (batch_id = 0)
        let catch_all_key = (0, 0);
        if self.catch_all_only() {
            if let Some(incident_id) = self.base.active_incidents.get(&catch_all_key) {
                let payload = self.base.create_resolve_payload();
                self.base.resolve_incident_with_payload(incident_id, &payload).await?;
                self.base.active_incidents.remove(&catch_all_key);
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Monitor for BatchProofTimeoutMonitor {
    type IncidentKey = (u64, u64);

    async fn create_incident(&self, key: &Self::IncidentKey) -> Result<String> {
        let (_, batch_id) = *key;
        self.open_incident(batch_id, Utc::now(), 0).await
    }

    async fn resolve_incident(&self, incident_id: &str) -> Result<()> {
        let payload = self.base.create_resolve_payload();
        self.base.resolve_incident_with_payload(incident_id, &payload).await
    }

    async fn check_health(&mut self) -> Result<()> {
        self.check_unproven_batches().await
    }

    async fn initialize(&mut self) -> Result<()> {
        // Check for existing incidents, using (0,0) as catch-all key
        self.base.check_existing_incidents((0, 0)).await
    }

    async fn run(mut self) -> Result<()> {
        self.initialize().await?;
        let interval_duration = self.get_interval();
        let mut interval = tokio::time::interval(interval_duration);
        loop {
            interval.tick().await;
            if let Err(e) = self.check_health().await {
                error!(error = %e, "monitoring check failed for BatchProofTimeoutMonitor");
            }
        }
    }

    fn get_interval(&self) -> Duration {
        self.base.interval
    }

    fn get_component_id(&self) -> &str {
        &self.base.component_id
    }

    fn get_client(&self) -> &IncidentClient {
        &self.base.client
    }

    fn get_clickhouse(&self) -> &ClickhouseReader {
        &self.base.clickhouse
    }
}

/// Monitors batches that take too long to be verified (> X hours after being posted).
/// Creates incidents for batches that have been posted but not verified within the time threshold.
#[derive(Debug)]
pub struct BatchVerifyTimeoutMonitor {
    /// Base monitor implementation
    base: BaseMonitor<u64>,
    /// Timeout threshold for batch verification
    verify_timeout: Duration,
}

impl BatchVerifyTimeoutMonitor {
    /// Creates a new `BatchVerifyTimeoutMonitor` with the given parameters.
    pub fn new(
        clickhouse: ClickhouseReader,
        client: IncidentClient,
        component_id: String,
        verify_timeout: Duration,
        interval: Duration,
    ) -> Self {
        Self { base: BaseMonitor::new(clickhouse, client, component_id, interval), verify_timeout }
    }

    /// Check if a specific batch has been verified
    async fn is_batch_verified(&self, batch_id: u64) -> Result<bool> {
        let verified_batch_ids = self.get_verified_batch_ids().await?;
        Ok(verified_batch_ids.contains(&batch_id))
    }

    /// Get all batch IDs that have been verified
    async fn get_verified_batch_ids(&self) -> Result<Vec<u64>> {
        self.base.clickhouse.get_verified_batch_ids().await
    }

    /// Creates an incident for an unverified batch
    async fn open_incident(
        &self,
        batch_id: u64,
        posted_at: DateTime<Utc>,
        age_hours: u64, // Or use `Duration` directly from `verify_timeout`
    ) -> Result<String> {
        let incident_start_time = posted_at + ChronoDuration::from_std(self.verify_timeout)?;

        let body = self.base.create_incident_payload(
            format!("Batch #{} Not Verified - Timeout", batch_id),
            format!(
                "Batch #{} has been waiting for verification for over {}h (threshold: {}h)",
                batch_id,
                age_hours, // This could be calculated more precisely or taken from verify_timeout
                self.verify_timeout.as_secs() / 3600
            ),
            incident_start_time,
        );

        let id = self.base.create_incident_with_payload(&body).await?;

        info!(
            incident_id = %id,
            batch_id = batch_id,
            "Created batch verify timeout incident"
        );

        Ok(id)
    }

    /// Create incidents for any unverified batches that exceed the timeout
    async fn add_new_unverified_incidents(
        &mut self,
        unverified_batches: &[(u64, u64, DateTime<Utc>)],
    ) -> Result<()> {
        for (_l1_block_number, batch_id, posted_at) in unverified_batches {
            let age_duration = Utc::now().signed_duration_since(*posted_at);
            if age_duration > ChronoDuration::from_std(self.verify_timeout)? &&
                !self.base.active_incidents.contains_key(batch_id)
            {
                debug!(
                    batch_id = batch_id,
                    posted_at = %posted_at,
                    age_hours = age_duration.num_hours(),
                    "Found unverified batch exceeding timeout",
                );
                let incident_id = self
                    .open_incident(*batch_id, *posted_at, age_duration.num_hours() as u64)
                    .await?;
                self.base.active_incidents.insert(*batch_id, incident_id);
            }
        }

        Ok(())
    }

    /// Resolve incidents for batches that have since been verified
    async fn resolve_verified_incidents(&mut self) -> Result<()> {
        let active_incidents_snapshot: Vec<(u64, String)> = self
            .base
            .active_incidents
            .iter()
            .map(|(id, incident)| (*id, incident.clone()))
            .collect();

        for (batch_id, incident_id) in active_incidents_snapshot {
            if batch_id == 0 {
                continue;
            }

            if self.is_batch_verified(batch_id).await? {
                debug!(
                    batch_id = batch_id,
                    incident_id = %incident_id,
                    "Batch is now verified, resolving incident immediately",
                );
                let payload = self.base.create_resolve_payload();
                self.base.resolve_incident_with_payload(&incident_id, &payload).await?;
                self.base.active_incidents.remove(&batch_id);
            } else {
                self.base.mark_unhealthy();
            }
        }

        Ok(())
    }

    /// Resolve the catch-all incident if no specific incidents remain
    async fn resolve_catch_all_if_clear(&mut self, none_left: bool) -> Result<()> {
        let catch_all_key = 0;
        if self.base.active_incidents.len() == 1 &&
            self.base.active_incidents.contains_key(&catch_all_key) &&
            none_left
        {
            if let Some(incident_id) = self.base.active_incidents.get(&catch_all_key) {
                info!(
                    incident_id = %incident_id,
                    "Resolving general batch verification timeout incident as all specific batches are clear or verified."
                );
                let payload = self.base.create_resolve_payload();
                self.base.resolve_incident_with_payload(incident_id, &payload).await?;
                self.base.active_incidents.remove(&catch_all_key);
            }
        }
        Ok(())
    }

    /// Check for batches that have not been verified within the timeout period
    async fn check_unverified_batches(&mut self) -> Result<()> {
        let cutoff_time = Utc::now() - ChronoDuration::from_std(self.verify_timeout)?;
        let unverified_batches =
            self.base.clickhouse.get_unverified_batches_older_than(cutoff_time).await?;

        debug!(
            "Found {} unverified batches older than {:?}",
            unverified_batches.len(),
            self.verify_timeout
        );

        self.add_new_unverified_incidents(&unverified_batches).await?;
        self.resolve_verified_incidents().await?;
        self.resolve_catch_all_if_clear(unverified_batches.is_empty()).await?;

        Ok(())
    }
}

#[async_trait]
impl Monitor for BatchVerifyTimeoutMonitor {
    type IncidentKey = u64;

    async fn create_incident(&self, key: &Self::IncidentKey) -> Result<String> {
        // For manual creation, we might not have a `posted_at` time easily,
        // so we use Utc::now() and an age of 0, or adjust as needed.
        self.open_incident(*key, Utc::now(), 0).await
    }

    async fn resolve_incident(&self, incident_id: &str) -> Result<()> {
        let payload = self.base.create_resolve_payload();
        self.base.resolve_incident_with_payload(incident_id, &payload).await
    }

    async fn check_health(&mut self) -> Result<()> {
        self.check_unverified_batches().await
    }

    async fn initialize(&mut self) -> Result<()> {
        // Check for existing incidents. Using 0 as a generic key for component-wide issues.
        self.base.check_existing_incidents(0).await
        // Potentially add an initial health check if needed, similar to other monitors
        // self.check_unverified_batches().await // Initial check
    }

    async fn run(mut self) -> Result<()> {
        self.initialize().await?;
        let interval_duration = self.get_interval();
        let mut interval = tokio::time::interval(interval_duration);
        loop {
            interval.tick().await;
            if let Err(e) = self.check_health().await {
                error!(error = %e, "monitoring check failed for BatchVerifyTimeoutMonitor");
            }
        }
    }

    fn get_interval(&self) -> Duration {
        self.base.interval
    }

    fn get_component_id(&self) -> &str {
        &self.base.component_id
    }

    fn get_client(&self) -> &IncidentClient {
        &self.base.client
    }

    fn get_clickhouse(&self) -> &ClickhouseReader {
        &self.base.clickhouse
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::Client as IncidentClient;
    use clickhouse::ClickhouseReader as ClickhouseInternalClient;
    use mockito::{Matcher, Server, ServerGuard};
    use std::time::Duration;
    use url::Url;

    // Helper to create a ClickhouseClient for tests
    fn mock_clickhouse_client() -> (ClickhouseInternalClient, ServerGuard) {
        let server = mockito::Server::new();
        let url = Url::parse(&server.url()).unwrap();
        let client = ClickhouseInternalClient::new(
            url,
            "test_db".to_owned(),
            "user".to_owned(),
            "pass".to_owned(),
        )
        .unwrap();
        (client, server)
    }

    // Helper to create an IncidentClient for tests
    fn mock_incident_client() -> (IncidentClient, ServerGuard) {
        let server = mockito::Server::new();
        let url = Url::parse(&server.url()).unwrap();
        let client = IncidentClient::with_base_url(
            "test_api_key".to_owned(),
            "test_page_id".to_owned(),
            url,
        );
        (client, server)
    }

    async fn mock_clickhouse_client_async() -> (ClickhouseInternalClient, ServerGuard) {
        let server = Server::new_async().await;
        let url = Url::parse(&server.url()).unwrap();
        let client = ClickhouseInternalClient::new(
            url,
            "test_db".to_owned(),
            "user".to_owned(),
            "pass".to_owned(),
        )
        .unwrap();
        (client, server)
    }

    #[tokio::test]
    async fn instatus_monitor_create_and_resolve_incident() {
        let (ch_client, _ch_server) = mock_clickhouse_client_async().await;
        let mut server = Server::new_async().await;

        let post_mock = server
            .mock("POST", "/v1/test_page_id/incidents")
            .match_header("authorization", "Bearer test_api_key")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_body(r#"{"id":"inc1"}"#)
            .create_async()
            .await;

        let put_mock = server
            .mock("PUT", "/v1/test_page_id/incidents/inc1")
            .match_header("authorization", "Bearer test_api_key")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_body("{}")
            .create_async()
            .await;

        let get_mock = server
            .mock("GET", "/v1/test_page_id/incidents")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_body("[]")
            .create_async()
            .await;

        let incident_client = IncidentClient::with_base_url(
            "test_api_key".into(),
            "test_page_id".into(),
            server.url().parse().unwrap(),
        );

        let monitor = InstatusMonitor::new(
            ch_client,
            incident_client,
            "comp1".to_owned(),
            Duration::from_secs(60),
            Duration::from_secs(1),
        );

        let id = monitor.create_incident(&()).await.unwrap();
        assert_eq!(id, "inc1");

        monitor.resolve_incident(&id).await.unwrap();

        post_mock.assert_async().await;
        put_mock.assert_async().await;
        get_mock.assert_async().await;
    }

    #[tokio::test]
    async fn instatus_l1_monitor_create_and_resolve_incident() {
        let (ch_client, _ch_server) = mock_clickhouse_client_async().await;
        let mut server = Server::new_async().await;

        let post_mock = server
            .mock("POST", "/v1/test_page_id/incidents")
            .match_header("authorization", "Bearer test_api_key")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_body(r#"{"id":"inc1"}"#)
            .create_async()
            .await;

        let put_mock = server
            .mock("PUT", "/v1/test_page_id/incidents/inc1")
            .match_header("authorization", "Bearer test_api_key")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_body("{}")
            .create_async()
            .await;

        let incident_client = IncidentClient::with_base_url(
            "test_api_key".into(),
            "test_page_id".into(),
            server.url().parse().unwrap(),
        );

        let monitor = InstatusL1Monitor::new(
            ch_client,
            incident_client,
            "comp1".to_owned(),
            Duration::from_secs(60),
            Duration::from_secs(1),
        );

        let id = monitor.create_incident(&()).await.unwrap();
        assert_eq!(id, "inc1");

        monitor.resolve_incident(&id).await.unwrap();

        post_mock.assert_async().await;
        put_mock.assert_async().await;
    }

    #[tokio::test]
    async fn instatus_monitor_handle_opens_and_resolves_incident() {
        let (ch_client, _ch_server) = mock_clickhouse_client_async().await;
        let mut server = Server::new_async().await;

        let post_mock = server
            .mock("POST", "/v1/test_page_id/incidents")
            .match_header("authorization", "Bearer test_api_key")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_body(r#"{"id":"inc1"}"#)
            .create_async()
            .await;

        let put_mock = server
            .mock("PUT", "/v1/test_page_id/incidents/inc1")
            .match_header("authorization", "Bearer test_api_key")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_body("{}")
            .create_async()
            .await;
        let get_mock = server
            .mock("GET", "/v1/test_page_id/incidents")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_body("[]")
            .create_async()
            .await;

        let incident_client = IncidentClient::with_base_url(
            "test_api_key".into(),
            "test_page_id".into(),
            server.url().parse().unwrap(),
        );

        let mut monitor = InstatusMonitor::new(
            ch_client,
            incident_client,
            "comp1".to_owned(),
            Duration::from_secs(60),
            Duration::from_secs(1),
        );

        let outdated = Utc::now() - ChronoDuration::seconds(120);
        monitor.handle(outdated).await.unwrap();
        assert_eq!(monitor.base.active_incidents.get(&()), Some(&"inc1".to_owned()));

        monitor.handle(Utc::now()).await.unwrap();
        assert!(monitor.base.active_incidents.is_empty());

        put_mock.assert_async().await;
        post_mock.assert_async().await;
        get_mock.assert_async().await;
    }

    #[tokio::test]
    async fn instatus_monitor_does_not_duplicate_incident() {
        let (ch_client, _ch_server) = mock_clickhouse_client_async().await;
        let mut server = Server::new_async().await;

        let list = serde_json::json!([
            {"id": "inc1", "components": [{"id": "comp1", "name": "comp1", "status": "MAJOROUTAGE"}]}
        ])
        .to_string();

        let get_mock = server
            .mock("GET", "/v1/test_page_id/incidents")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_body(list)
            .create_async()
            .await;

        let post_mock =
            server.mock("POST", "/v1/test_page_id/incidents").expect(0).create_async().await;

        let incident_client = IncidentClient::with_base_url(
            "test_api_key".into(),
            "test_page_id".into(),
            server.url().parse().unwrap(),
        );

        let mut monitor = InstatusMonitor::new(
            ch_client,
            incident_client,
            "comp1".to_owned(),
            Duration::from_secs(60),
            Duration::from_secs(1),
        );

        let outdated = Utc::now() - ChronoDuration::seconds(120);
        monitor.handle(outdated).await.unwrap();
        assert_eq!(monitor.base.active_incidents.get(&()), Some(&"inc1".to_owned()));

        get_mock.assert_async().await;
        post_mock.assert_async().await;
    }

    #[test]
    fn filter_new_batches_only_returns_untracked() {
        let (ch_client, _ch_server) = mock_clickhouse_client();
        let (incident_client, _incident_server) = mock_incident_client();
        let mut monitor = BatchProofTimeoutMonitor::new(
            ch_client,
            incident_client,
            "comp".to_owned(),
            Duration::from_secs(1),
            Duration::from_secs(1),
        );
        monitor.base.active_incidents.insert((1, 1), "id".to_owned());
        let now = Utc::now();
        let batches = vec![(1, 1, now), (2, 2, now)];
        let filtered = monitor.filter_new_batches(&batches);
        assert_eq!(filtered, vec![(2, 2, now)]);
    }

    #[test]
    fn catch_all_only_true_only_for_catch_all() {
        let (ch_client, _ch_server) = mock_clickhouse_client();
        let (incident_client, _incident_server) = mock_incident_client();
        let mut monitor = BatchProofTimeoutMonitor::new(
            ch_client,
            incident_client,
            "comp".to_owned(),
            Duration::from_secs(1),
            Duration::from_secs(1),
        );
        assert!(!monitor.catch_all_only());
        monitor.base.active_incidents.insert((0, 0), "id".to_owned());
        assert!(monitor.catch_all_only());
        monitor.base.active_incidents.insert((1, 1), "other".to_owned());
        assert!(!monitor.catch_all_only());
    }

    #[tokio::test]
    async fn hekla_rpc_monitor_resolves_when_not_syncing() {
        let (ch_client, _ch_server) = mock_clickhouse_client_async().await;
        let mut incident_server = Server::new_async().await;

        let put_mock = incident_server
            .mock("PUT", "/v1/test_page_id/incidents/inc1")
            .match_header("authorization", "Bearer test_api_key")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_body("{}")
            .create_async()
            .await;

        let incident_client = IncidentClient::with_base_url(
            "test_api_key".into(),
            "test_page_id".into(),
            incident_server.url().parse().unwrap(),
        );

        let mut rpc_server = Server::new_async().await;
        rpc_server
            .mock("POST", "/")
            .with_status(200)
            .with_body(r#"{"jsonrpc":"2.0","id":1,"result":false}"#)
            .create_async()
            .await;

        let mut monitor = HeklaRpcMonitor::new(
            ch_client,
            incident_client,
            "comp".to_owned(),
            rpc_server.url().parse().unwrap(),
            Duration::from_secs(1),
        );
        monitor.base.active_incidents.insert((), "inc1".to_owned());

        monitor.check_health().await.unwrap();
        assert!(monitor.base.active_incidents.is_empty());

        put_mock.assert_async().await;
    }

    #[tokio::test]
    async fn hekla_rpc_monitor_opens_on_syncing() {
        let (ch_client, _ch_server) = mock_clickhouse_client_async().await;
        let mut incident_server = Server::new_async().await;

        let post_mock = incident_server
            .mock("POST", "/v1/test_page_id/incidents")
            .match_header("authorization", "Bearer test_api_key")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_body(r#"{"id":"inc1"}"#)
            .create_async()
            .await;
        let get_mock = incident_server
            .mock("GET", "/v1/test_page_id/incidents")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_body("[]")
            .create_async()
            .await;

        let incident_client = IncidentClient::with_base_url(
            "test_api_key".into(),
            "test_page_id".into(),
            incident_server.url().parse().unwrap(),
        );

        let mut rpc_server = Server::new_async().await;
        rpc_server
            .mock("POST", "/")
            .with_status(200)
            .with_body(r#"{"jsonrpc":"2.0","id":1,"result":{"startingBlock":"0x1"}}"#)
            .create_async()
            .await;

        let mut monitor = HeklaRpcMonitor::new(
            ch_client,
            incident_client,
            "comp".to_owned(),
            rpc_server.url().parse().unwrap(),
            Duration::from_secs(1),
        );

        monitor.check_health().await.unwrap();
        assert_eq!(monitor.base.active_incidents.get(&()), Some(&"inc1".to_owned()));

        post_mock.assert_async().await;
        get_mock.assert_async().await;
    }
}
