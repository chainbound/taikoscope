use crate::{
    base_monitor::{BaseMonitor, Monitor},
    client::Client as IncidentClient,
};
use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use clickhouse::ClickhouseClient;
use eyre::Result;
use serde::Serialize;
use std::time::Duration;
use tracing::{debug, error, info, warn};

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
        clickhouse: ClickhouseClient,
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
                if let Err(e) = self.handle(batch_ts, l2_ts).await {
                    error!(%e, "handling new batch event status");
                }
            }
            (Ok(None), Ok(Some(_))) => {
                warn!("no batch event timestamp available this tick for batch monitor")
            }
            (_, Ok(None)) => {
                warn!("no L2 head timestamp available this tick for batch monitor")
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

    fn get_clickhouse(&self) -> &ClickhouseClient {
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
        clickhouse: ClickhouseClient,
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

    fn get_clickhouse(&self) -> &ClickhouseClient {
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
        clickhouse: ClickhouseClient,
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
        for (l1_block_number, batch_id, posted_at) in &unproven_batches {
            let key = (*l1_block_number, *batch_id);
            let age_hours = Utc::now().signed_duration_since(*posted_at).num_hours();
            if !self.base.active_incidents.contains_key(&key) {
                debug!(
                    batch_id = batch_id,
                    posted_at = %posted_at,
                    age_hours = age_hours,
                    "Found unproven batch exceeding timeout"
                );
                let incident_id =
                    self.open_incident(*batch_id, *posted_at, age_hours as u64).await?;
                self.base.active_incidents.insert(key, incident_id);
            }
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
        if self.base.active_incidents.len() == 1 &&
            self.base.active_incidents.contains_key(&catch_all_key)
        {
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

    fn get_clickhouse(&self) -> &ClickhouseClient {
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
        clickhouse: ClickhouseClient,
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

        // Create incidents for new unverified batches
        for (_l1_block_number, batch_id, posted_at) in &unverified_batches {
            let age_duration = Utc::now().signed_duration_since(*posted_at);
            if age_duration > ChronoDuration::from_std(self.verify_timeout)? &&
                !self.base.active_incidents.contains_key(batch_id)
            {
                debug!(
                    batch_id = batch_id,
                    posted_at = %posted_at,
                    age_hours = age_duration.num_hours(),
                    "Found unverified batch exceeding timeout"
                );
                let incident_id = self
                    .open_incident(*batch_id, *posted_at, age_duration.num_hours() as u64)
                    .await?;
                self.base.active_incidents.insert(*batch_id, incident_id);
            }
        }

        // Take a snapshot of active incidents to avoid concurrent immutable/mutable borrows
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
            let is_verified = self.is_batch_verified(batch_id).await?;
            if is_verified {
                debug!(
                    batch_id = batch_id,
                    incident_id = %incident_id,
                    "Batch is now verified, resolving incident immediately"
                );
                let payload = self.base.create_resolve_payload();
                self.base.resolve_incident_with_payload(&incident_id, &payload).await?;
                self.base.active_incidents.remove(&batch_id);
            } else {
                self.base.mark_unhealthy();
            }
        }

        // Handle the catch-all incident (batch_id = 0) if all specific batch incidents are cleared
        if self.base.active_incidents.len() == 1 &&
            self.base.active_incidents.contains_key(&0) &&
            unverified_batches.is_empty()
        {
            // Or a more robust check if all specific incidents were just resolved
            if let Some(incident_id) = self.base.active_incidents.get(&0) {
                info!(incident_id = %incident_id, "Resolving general batch verification timeout incident as all specific batches are clear or verified.");
                let payload = self.base.create_resolve_payload();
                self.base.resolve_incident_with_payload(incident_id, &payload).await?;
                self.base.active_incidents.remove(&0);
            }
        }
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

    fn get_clickhouse(&self) -> &ClickhouseClient {
        &self.base.clickhouse
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::Client as ActualIncidentClient; // Renamed for clarity
    use crate::base_monitor::BaseMonitor; // For instantiating InstatusL1Monitor
    use clickhouse::ClickhouseClient as ActualClickhouseClient; // Renamed for clarity
    use mockall::mock;
    use mockito::ServerGuard; // Keep for existing tests if any, or remove if replacing all
    use std::time::Duration;
    use chrono::{DateTime, Utc, TimeZone};
    use url::Url; // Keep for existing tests

    // --- Mocks using mockall for InstatusL1Monitor unit tests ---
    mock! {
        pub ClickhouseClient { // Mock for the ClickhouseClient used by monitors
            // Define methods of ActualClickhouseClient that InstatusL1Monitor uses:
            // - get_last_batch_time() -> Result<Option<DateTime<Utc>>>
            // - get_last_l2_head_time() -> Result<Option<DateTime<Utc>>>
            // Add pub if methods are public in the actual client
            pub async fn get_last_batch_time(&self) -> Result<Option<DateTime<Utc>>>;
            pub async fn get_last_l2_head_time(&self) -> Result<Option<DateTime<Utc>>>;

            // Methods used by BatchProofTimeoutMonitor and BatchVerifyTimeoutMonitor
            pub async fn get_proved_batch_ids(&self) -> Result<Vec<u64>>;
            pub async fn get_unproved_batches_older_than(&self, cutoff: DateTime<Utc>) -> Result<Vec<(u64, u64, DateTime<Utc>)>>; // l1_block_number, batch_id, posted_at
            pub async fn get_verified_batch_ids(&self) -> Result<Vec<u64>>; // For BatchVerifyTimeoutMonitor later
            pub async fn get_unverified_batches_older_than(&self, cutoff: DateTime<Utc>) -> Result<Vec<(u64, u64, DateTime<Utc>)>>; // For BatchVerifyTimeoutMonitor later


            // Constructor mock, if needed for BaseMonitor::new
            // pub fn new(url: &str, database: String, user: String, pass: String) -> Result<Self> where Self: Sized;
        }
    }

    mock! {
        pub IncidentClient { // Mock for the IncidentClient (aliased as Client)
            // Define methods of ActualIncidentClient that BaseMonitor (used by InstatusL1Monitor) uses:
            // - create_incident(payload: &NewIncident) -> Result<String>
            // - resolve_incident(id: &str, payload: &ResolveIncident) -> Result<()>
            // - open_incident(component_id: &str) -> Result<Option<String>>
            pub async fn create_incident(&self, incident: &NewIncident) -> Result<String>;
            pub async fn resolve_incident(&self, incident_id: &str, resolution: &ResolveIncident) -> Result<()>;
            pub async fn open_incident(&self, component_id: &str) -> Result<Option<String>>;

            // Constructor mock, if needed.
            // pub fn new(api_key: String, page_id: String) -> Self where Self: Sized;
            // pub fn with_base_url(api_key: String, page_id: String, base_url: Url) -> Self where Self: Sized;
        }
    }

    // Helper to create InstatusL1Monitor with mock clients
    fn instatus_l1_monitor_with_mocks(
        mock_ch_client: MockClickhouseClient,
        mock_incident_client: MockIncidentClient,
        component_id: String,
        threshold_seconds: u64,
        interval_seconds: u64,
    ) -> InstatusL1Monitor {
        InstatusL1Monitor::new(
            mock_ch_client.into(), // Convert mock to the actual type expected by BaseMonitor
            mock_incident_client.into(),
            component_id,
            Duration::from_secs(threshold_seconds),
            Duration::from_secs(interval_seconds),
        )
    }

    // Helper to create BatchVerifyTimeoutMonitor with mock clients
    fn batch_verify_timeout_monitor_with_mocks(
        mock_ch_client: MockClickhouseClient,
        mock_incident_client: MockIncidentClient,
        component_id: String,
        verify_timeout_hours: u64,
        interval_seconds: u64,
    ) -> BatchVerifyTimeoutMonitor {
        BatchVerifyTimeoutMonitor::new(
            mock_ch_client.into(),
            mock_incident_client.into(),
            component_id,
            Duration::from_secs(verify_timeout_hours * 60 * 60),
            Duration::from_secs(interval_seconds),
        )
    }

    // Helper to create BatchProofTimeoutMonitor with mock clients
    fn batch_proof_timeout_monitor_with_mocks(
        mock_ch_client: MockClickhouseClient,
        mock_incident_client: MockIncidentClient,
        component_id: String,
        proof_timeout_hours: u64,
        interval_seconds: u64,
    ) -> BatchProofTimeoutMonitor {
        BatchProofTimeoutMonitor::new(
            mock_ch_client.into(),
            mock_incident_client.into(),
            component_id,
            Duration::from_secs(proof_timeout_hours * 60 * 60),
            Duration::from_secs(interval_seconds),
        )
    }

    // Helper to create InstatusMonitor with mock clients
    fn instatus_monitor_with_mocks(
        mock_ch_client: MockClickhouseClient,
        mock_incident_client: MockIncidentClient,
        component_id: String,
        threshold_seconds: u64,
        interval_seconds: u64,
    ) -> InstatusMonitor {
        InstatusMonitor::new(
            mock_ch_client.into(),
            mock_incident_client.into(),
            component_id,
            Duration::from_secs(threshold_seconds),
            Duration::from_secs(interval_seconds),
        )
    }


    // --- Existing mockito helpers (can be kept if other tests use them) ---

    // --- Tests for InstatusL1Monitor ---

    // Helper to create a DateTime<Utc> from seconds ago
    fn time_secs_ago(secs: i64) -> DateTime<Utc> {
        Utc::now() - ChronoDuration::seconds(secs)
    }

    #[tokio::test]
    async fn test_handle_scenario1_no_incident_batch_unhealthy_l2_healthy_opens_incident() {
        let mut mock_ch = MockClickhouseClient::new(); // Not used directly by handle, but BaseMonitor needs it
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "l1_test_component".to_string();
        let threshold_seconds = 60;
        let interval_seconds = 30;
        let expected_incident_id = "incident_opened_123".to_string();

        // Expectations
        let expected_id_clone = expected_incident_id.clone();
        mock_incident.expect_create_incident()
            .times(1)
            .returning(move |_payload| Ok(expected_id_clone.clone()));
        // get_last_batch_time and get_last_l2_head_time are not called by `handle` directly,
        // but by check_batch_and_l2 which calls handle. For direct handle test, we pass times.

        let mut monitor = instatus_l1_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            threshold_seconds,
            interval_seconds,
        );

        assert!(monitor.base.active_incidents.is_empty(), "Should start with no active incidents");

        let last_batch_time = time_secs_ago(threshold_seconds + 10); // Older than threshold -> unhealthy
        let last_l2_time = time_secs_ago(threshold_seconds - 10);   // Younger than threshold -> healthy

        let result = monitor.handle(last_batch_time, last_l2_time).await;

        assert!(result.is_ok());
        assert_eq!(monitor.base.active_incidents.len(), 1, "Incident should be opened");
        assert_eq!(monitor.base.active_incidents.get(&()), Some(&expected_incident_id), "Correct incident ID should be stored");
    }

    #[tokio::test]
    async fn test_handle_scenario2_active_incident_batch_unhealthy_remains_unhealthy() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "l1_test_s2".to_string();
        let threshold_seconds = 60;
        let existing_incident_id = "incident_active_456".to_string();

        // Expectations: No client calls to create or resolve
        mock_incident.expect_create_incident().never();
        mock_incident.expect_resolve_incident().never();

        let mut monitor = instatus_l1_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            threshold_seconds,
            30, // interval
        );

        // Setup active incident
        monitor.base.active_incidents.insert((), existing_incident_id.clone());

        // Batch unhealthy (age > threshold), L2 can be anything (e.g., also unhealthy for this case)
        let last_batch_time = time_secs_ago(threshold_seconds + 20);
        let last_l2_time = time_secs_ago(threshold_seconds + 5); // L2 also unhealthy

        let result = monitor.handle(last_batch_time, last_l2_time).await;

        assert!(result.is_ok());
        assert_eq!(monitor.base.active_incidents.len(), 1, "Incident should remain active");
        assert_eq!(monitor.base.active_incidents.get(&()), Some(&existing_incident_id), "Existing incident ID should still be stored");
        // BaseMonitor::mark_unhealthy() is called, which is a no-op. We're verifying no other side effects.
    }

    #[tokio::test]
    async fn test_handle_scenario3_active_incident_batch_healthy_resolves_incident() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "l1_test_s3".to_string();
        let threshold_seconds = 60;
        let existing_incident_id = "incident_to_resolve_789".to_string();

        // Expectations: resolve_incident should be called
        mock_incident.expect_resolve_incident()
            .withf(move |id, _payload| id == existing_incident_id)
            .times(1)
            .returning(|_, _| Ok(()));
        mock_incident.expect_create_incident().never();


        let mut monitor = instatus_l1_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            threshold_seconds,
            30, // interval
        );

        // Setup active incident
        monitor.base.active_incidents.insert((), existing_incident_id.clone());

        // Batch healthy (age < threshold)
        let last_batch_time = time_secs_ago(threshold_seconds - 10); // Younger than threshold
        let last_l2_time = time_secs_ago(threshold_seconds - 20);   // L2 also healthy

        let result = monitor.handle(last_batch_time, last_l2_time).await;

        assert!(result.is_ok());
        assert!(monitor.base.active_incidents.is_empty(), "Incident should be resolved and removed");
    }

    #[tokio::test]
    async fn test_handle_scenario4_no_incident_both_healthy_no_action() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "l1_test_s4".to_string();
        let threshold_seconds = 60;

        // Expectations: No client calls
        mock_incident.expect_create_incident().never();
        mock_incident.expect_resolve_incident().never();

        let mut monitor = instatus_l1_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            threshold_seconds,
            30, // interval
        );

        assert!(monitor.base.active_incidents.is_empty(), "Should start with no active incidents");

        // Both batch and L2 healthy (age < threshold)
        let last_batch_time = time_secs_ago(threshold_seconds - 30);
        let last_l2_time = time_secs_ago(threshold_seconds - 25);

        let result = monitor.handle(last_batch_time, last_l2_time).await;

        assert!(result.is_ok());
        assert!(monitor.base.active_incidents.is_empty(), "No incident should be opened");
    }

    #[tokio::test]
    async fn test_handle_scenario5_no_incident_both_unhealthy_no_action() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "l1_test_s5".to_string();
        let threshold_seconds = 60;

        // Expectations: No client calls
        mock_incident.expect_create_incident().never();
        mock_incident.expect_resolve_incident().never();

        let mut monitor = instatus_l1_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            threshold_seconds,
            30, // interval
        );

        assert!(monitor.base.active_incidents.is_empty(), "Should start with no active incidents");

        // Both batch and L2 unhealthy (age > threshold)
        let last_batch_time = time_secs_ago(threshold_seconds + 40);
        let last_l2_time = time_secs_ago(threshold_seconds + 35);

        let result = monitor.handle(last_batch_time, last_l2_time).await;

        assert!(result.is_ok());
        assert!(monitor.base.active_incidents.is_empty(), "No incident should be opened when L2 is also unhealthy");
    }

    #[tokio::test]
    async fn test_initialize_no_existing_incident() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "l1_init_test_no_existing".to_string();
        let threshold_seconds = 60;

        // Expect open_incident to be called by base.check_existing_incidents
        let comp_id_clone = component_id.clone();
        mock_incident.expect_open_incident()
            .withf(move |id: &str| id == comp_id_clone) // BaseMonitor calls with its component_id
            .times(1)
            .returning(|_| Ok(None)); // No existing incident

        // check_initial_health is called. If no active incidents, it does not query CH.
        // So, no CH client calls are expected here.
        mock_ch.expect_get_last_batch_time().never();
        mock_ch.expect_get_last_l2_head_time().never();


        let mut monitor = instatus_l1_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            threshold_seconds,
            30, // interval
        );

        let result = monitor.initialize().await;
        assert!(result.is_ok());
        // Mockall automatically verifies that expectations were met (open_incident called once, CH calls never)
    }

    #[tokio::test]
    async fn test_initialize_with_existing_incident_resolves_if_healthy() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "l1_init_test_existing_resolves".to_string();
        let threshold_seconds = 60;
        let existing_incident_id = "existing_id_for_init_resolve".to_string();

        // 1. base.check_existing_incidents finds an incident
        let comp_id_clone1 = component_id.clone();
        let existing_id_clone1 = existing_incident_id.clone();
        mock_incident.expect_open_incident()
            .withf(move |id: &str| id == comp_id_clone1)
            .times(1)
            .returning(move |_| Ok(Some(existing_id_clone1.clone())));

        // 2. check_initial_health calls CH, gets healthy times
        let healthy_batch_time = time_secs_ago(threshold_seconds - 10);
        let healthy_l2_time = time_secs_ago(threshold_seconds - 15);
        mock_ch.expect_get_last_batch_time()
            .times(1)
            .returning(move || Ok(Some(healthy_batch_time)));
        mock_ch.expect_get_last_l2_head_time()
            .times(1)
            .returning(move || Ok(Some(healthy_l2_time)));

        // 3. handle (called by check_initial_health) resolves the incident
        let existing_id_clone2 = existing_incident_id.clone();
        mock_incident.expect_resolve_incident()
            .withf(move |id, _payload| id == existing_id_clone2)
            .times(1)
            .returning(|_, _| Ok(()));

        let mut monitor = instatus_l1_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            threshold_seconds,
            30, // interval
        );

        let result = monitor.initialize().await;
        assert!(result.is_ok());
        assert!(monitor.base.active_incidents.is_empty(), "Existing incident should be resolved and removed");
    }

    #[tokio::test]
    async fn test_initialize_with_existing_incident_remains_if_unhealthy() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "l1_init_test_existing_unhealthy".to_string();
        let threshold_seconds = 60;
        let existing_incident_id = "existing_id_for_init_unhealthy".to_string();

        // 1. base.check_existing_incidents finds an incident
        let comp_id_clone1 = component_id.clone();
        let existing_id_clone1 = existing_incident_id.clone();
        mock_incident.expect_open_incident()
            .withf(move |id: &str| id == comp_id_clone1)
            .times(1)
            .returning(move |_| Ok(Some(existing_id_clone1.clone())));

        // 2. check_initial_health calls CH, gets unhealthy times
        let unhealthy_batch_time = time_secs_ago(threshold_seconds + 20); // Older than threshold
        let unhealthy_l2_time = time_secs_ago(threshold_seconds + 25);   // Older than threshold
        mock_ch.expect_get_last_batch_time()
            .times(1)
            .returning(move || Ok(Some(unhealthy_batch_time)));
        mock_ch.expect_get_last_l2_head_time()
            .times(1)
            .returning(move || Ok(Some(unhealthy_l2_time)));

        // 3. handle (called by check_initial_health) should not resolve. No create either.
        mock_incident.expect_resolve_incident().never();
        mock_incident.expect_create_incident().never();


        let mut monitor = instatus_l1_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            threshold_seconds,
            30, // interval
        );

        let result = monitor.initialize().await;
        assert!(result.is_ok());
        assert_eq!(monitor.base.active_incidents.len(), 1, "Incident should remain");
        assert_eq!(monitor.base.active_incidents.get(&()), Some(&existing_incident_id), "Existing incident ID should still be stored");
    }

    #[tokio::test]
    async fn test_check_initial_health_active_incident_resolves_if_healthy() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "l1_check_initial_healthy_resolve".to_string();
        let threshold_seconds = 60;
        let existing_incident_id = "existing_id_for_check_initial_resolve".to_string();

        // Pre-condition: An incident is already active
        // (This would have been set by base.check_existing_incidents in a real initialize call)

        // 1. check_initial_health calls CH, gets healthy times
        let healthy_batch_time = time_secs_ago(threshold_seconds - 10);
        let healthy_l2_time = time_secs_ago(threshold_seconds - 15);
        mock_ch.expect_get_last_batch_time()
            .times(1)
            .returning(move || Ok(Some(healthy_batch_time)));
        mock_ch.expect_get_last_l2_head_time()
            .times(1)
            .returning(move || Ok(Some(healthy_l2_time)));

        // 2. handle (called by check_initial_health) resolves the incident
        let existing_id_clone = existing_incident_id.clone();
        mock_incident.expect_resolve_incident()
            .withf(move |id, _payload| id == existing_id_clone)
            .times(1)
            .returning(|_, _| Ok(()));
        mock_incident.expect_create_incident().never(); // No new incident should be created


        let mut monitor = instatus_l1_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            threshold_seconds,
            30, // interval
        );
        // Manually set active incident as check_existing_incidents is not called here
        monitor.base.active_incidents.insert((), existing_incident_id.clone());


        let result = monitor.check_initial_health().await;
        assert!(result.is_ok());
        assert!(monitor.base.active_incidents.is_empty(), "Existing incident should be resolved and removed by check_initial_health via handle");
    }

    #[tokio::test]
    async fn test_check_initial_health_active_incident_remains_if_unhealthy() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "l1_check_initial_unhealthy_remain".to_string();
        let threshold_seconds = 60;
        let existing_incident_id = "existing_id_for_check_initial_unhealthy".to_string();

        // 1. check_initial_health calls CH, gets unhealthy times
        let unhealthy_batch_time = time_secs_ago(threshold_seconds + 20);
        let unhealthy_l2_time = time_secs_ago(threshold_seconds + 25);
        mock_ch.expect_get_last_batch_time()
            .times(1)
            .returning(move || Ok(Some(unhealthy_batch_time)));
        mock_ch.expect_get_last_l2_head_time()
            .times(1)
            .returning(move || Ok(Some(unhealthy_l2_time)));

        // 2. handle (called by check_initial_health) should not resolve or create.
        mock_incident.expect_resolve_incident().never();
        mock_incident.expect_create_incident().never();


        let mut monitor = instatus_l1_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            threshold_seconds,
            30, // interval
        );
        monitor.base.active_incidents.insert((), existing_incident_id.clone());


        let result = monitor.check_initial_health().await;
        assert!(result.is_ok());
        assert_eq!(monitor.base.active_incidents.len(), 1, "Incident should remain active");
        assert_eq!(monitor.base.active_incidents.get(&()), Some(&existing_incident_id), "Correct incident ID should still be stored");
    }

    // --- Tests for Monitor trait methods on InstatusL1Monitor ---

    #[tokio::test]
    async fn test_instatus_l1_monitor_create_incident() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident_client = MockIncidentClient::new();
        let component_id = "l1_trait_create_test".to_string();
        let threshold_seconds = 60;
        let expected_incident_id = "trait_created_incident_1".to_string();

        let expected_id_clone = expected_incident_id.clone();
        mock_incident_client.expect_create_incident()
            .withf(move |p: &NewIncident| {
                p.name == "No BatchProposed events - Possible Outage" &&
                p.message.contains(&format!("No batch event for {}s", threshold_seconds))
            })
            .times(1)
            .returning(move |_| Ok(expected_id_clone.clone()));

        let monitor = instatus_l1_monitor_with_mocks(
            mock_ch,
            mock_incident_client,
            component_id,
            threshold_seconds,
            30, // interval
        );

        // The key for InstatusL1Monitor is ()
        let result = monitor.create_incident(&()).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_incident_id);
        // Note: create_incident on the trait doesn't add to monitor.base.active_incidents itself.
        // That's typically done by the calling logic (e.g., in handle).
    }

    #[tokio::test]
    async fn test_instatus_l1_monitor_resolve_incident() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident_client = MockIncidentClient::new();
        let component_id = "l1_trait_resolve_test".to_string();
        let incident_id_to_resolve = "trait_resolving_incident_2".to_string();

        let id_clone = incident_id_to_resolve.clone();
        mock_incident_client.expect_resolve_incident()
            .withf(move |id: &str, p: &ResolveIncident| {
                id == id_clone && p.status == IncidentState::Resolved
            })
            .times(1)
            .returning(|_, _| Ok(()));

        let monitor = instatus_l1_monitor_with_mocks(
            mock_ch,
            mock_incident_client,
            component_id,
            60, // threshold
            30, // interval
        );

        let result = monitor.resolve_incident(&incident_id_to_resolve).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_instatus_l1_monitor_check_health_opens_incident_on_unhealthy() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident_client = MockIncidentClient::new();
        let component_id = "l1_trait_check_health_open".to_string();
        let threshold_seconds = 60;
        let expected_incident_id = "check_health_opened_3".to_string();

        // 1. CH returns batch unhealthy, L2 healthy
        let unhealthy_batch_time = time_secs_ago(threshold_seconds + 30);
        let healthy_l2_time = time_secs_ago(threshold_seconds - 30);
        mock_ch.expect_get_last_batch_time()
            .times(1)
            .returning(move || Ok(Some(unhealthy_batch_time)));
        mock_ch.expect_get_last_l2_head_time()
            .times(1)
            .returning(move || Ok(Some(healthy_l2_time)));

        // 2. Handle (called by check_batch_and_l2) should open an incident
        let expected_id_clone = expected_incident_id.clone();
        mock_incident_client.expect_create_incident()
            .times(1)
            .returning(move |_| Ok(expected_id_clone.clone()));


        let mut monitor = instatus_l1_monitor_with_mocks(
            mock_ch,
            mock_incident_client,
            component_id,
            threshold_seconds,
            30, // interval
        );

        let result = monitor.check_health().await;
        assert!(result.is_ok());
        assert_eq!(monitor.base.active_incidents.len(), 1, "Incident should be opened by check_health");
        assert_eq!(monitor.base.active_incidents.get(&()), Some(&expected_incident_id));
    }

    // --- Tests for InstatusMonitor ---

    #[tokio::test]
    async fn test_instatusmonitor_handle_scenario1_no_incident_l2_unhealthy_opens_incident() {
        let mut mock_ch = MockClickhouseClient::new(); // Not used by InstatusMonitor::handle directly
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "instatus_test_component".to_string();
        let threshold_seconds = 120; // e.g., 2 minutes
        let expected_incident_id = "incident_l2_down_1".to_string();

        // Expectations for IncidentClient: create_incident should be called
        let expected_id_clone = expected_incident_id.clone();
        mock_incident.expect_create_incident()
            .withf(move |p: &NewIncident| {
                p.name == "No L2 head events - Possible Outage" &&
                p.message.contains(&format!("No L2 head event for {}s", threshold_seconds))
            })
            .times(1)
            .returning(move |_| Ok(expected_id_clone.clone()));

        let mut monitor = instatus_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            threshold_seconds,
            30, // interval
        );

        assert!(monitor.base.active_incidents.is_empty(), "Should start with no active incidents");

        // L2 event delayed (unhealthy)
        let last_l2_time = time_secs_ago(threshold_seconds + 30); // 30s older than threshold

        let result = monitor.handle(last_l2_time).await;

        assert!(result.is_ok());
        assert_eq!(monitor.base.active_incidents.len(), 1, "Incident should be opened");
        assert_eq!(monitor.base.active_incidents.get(&()), Some(&expected_incident_id), "Correct incident ID should be stored");
    }

    #[tokio::test]
    async fn test_instatusmonitor_handle_scenario2_active_incident_l2_unhealthy_remains_unhealthy() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "instatus_test_s2".to_string();
        let threshold_seconds = 120;
        let existing_incident_id = "incident_l2_still_down_2".to_string();

        // Expectations: No client calls to create or resolve
        mock_incident.expect_create_incident().never();
        mock_incident.expect_resolve_incident().never();

        let mut monitor = instatus_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            threshold_seconds,
            30, // interval
        );

        // Setup active incident
        monitor.base.active_incidents.insert((), existing_incident_id.clone());

        // L2 event still delayed (unhealthy)
        let last_l2_time = time_secs_ago(threshold_seconds + 45); // 45s older than threshold

        let result = monitor.handle(last_l2_time).await;

        assert!(result.is_ok());
        assert_eq!(monitor.base.active_incidents.len(), 1, "Incident should remain active");
        assert_eq!(monitor.base.active_incidents.get(&()), Some(&existing_incident_id), "Existing incident ID should still be stored");
        // BaseMonitor::mark_unhealthy() is called, which is a no-op. We're verifying no other side effects.
    }

    #[tokio::test]
    async fn test_instatusmonitor_handle_scenario3_active_incident_l2_healthy_resolves_incident() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "instatus_test_s3".to_string();
        let threshold_seconds = 120;
        let existing_incident_id = "incident_l2_recovers_3".to_string();

        // Expectations: resolve_incident should be called
        mock_incident.expect_resolve_incident()
            .withf(move |id, _payload| id == existing_incident_id)
            .times(1)
            .returning(|_, _| Ok(()));
        mock_incident.expect_create_incident().never(); // No new incident

        let mut monitor = instatus_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            threshold_seconds,
            30, // interval
        );

        // Setup active incident
        monitor.base.active_incidents.insert((), existing_incident_id.clone());

        // L2 event now recent (healthy)
        let last_l2_time = time_secs_ago(threshold_seconds - 60); // 60s younger than threshold

        let result = monitor.handle(last_l2_time).await;

        assert!(result.is_ok());
        assert!(monitor.base.active_incidents.is_empty(), "Incident should be resolved and removed");
    }

    #[tokio::test]
    async fn test_instatusmonitor_handle_scenario4_no_incident_l2_healthy_no_action() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "instatus_test_s4".to_string();
        let threshold_seconds = 120;

        // Expectations: No client calls
        mock_incident.expect_create_incident().never();
        mock_incident.expect_resolve_incident().never();

        let mut monitor = instatus_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            threshold_seconds,
            30, // interval
        );

        assert!(monitor.base.active_incidents.is_empty(), "Should start with no active incidents");

        // L2 event recent (healthy)
        let last_l2_time = time_secs_ago(threshold_seconds - 90); // 90s younger than threshold

        let result = monitor.handle(last_l2_time).await;

        assert!(result.is_ok());
        assert!(monitor.base.active_incidents.is_empty(), "No incident should be opened");
    }

    #[tokio::test]
    async fn test_instatusmonitor_initialize_no_existing_incident() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "instatus_init_no_existing".to_string();
        let threshold_seconds = 120;

        // Expect open_incident from base.check_existing_incidents
        let comp_id_clone = component_id.clone();
        mock_incident.expect_open_incident()
            .withf(move |id: &str| id == comp_id_clone)
            .times(1)
            .returning(|_| Ok(None)); // No existing incident

        // InstatusMonitor::check_initial_health, if no active_incidents, does nothing.
        mock_ch.expect_get_last_l2_head_time().never();

        let mut monitor = instatus_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            threshold_seconds,
            30, // interval
        );

        let result = monitor.initialize().await;
        assert!(result.is_ok());
        // mockall verifies expectations automatically
    }

    #[tokio::test]
    async fn test_instatusmonitor_initialize_existing_incident_resolves_if_healthy() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "instatus_init_existing_healthy".to_string();
        let threshold_seconds = 120;
        let existing_incident_id = "existing_l2_incident_resolve".to_string();

        // 1. base.check_existing_incidents finds an incident
        let comp_id_clone = component_id.clone();
        let existing_id_clone1 = existing_incident_id.clone();
        mock_incident.expect_open_incident()
            .withf(move |id: &str| id == comp_id_clone)
            .times(1)
            .returning(move |_| Ok(Some(existing_id_clone1.clone())));

        // 2. check_initial_health calls CH, gets healthy L2 time
        let healthy_l2_time = time_secs_ago(threshold_seconds - 30); // Younger than threshold
        mock_ch.expect_get_last_l2_head_time()
            .times(1)
            .returning(move || Ok(Some(healthy_l2_time)));

        // 3. handle (called by check_initial_health) resolves the incident
        let existing_id_clone2 = existing_incident_id.clone();
        mock_incident.expect_resolve_incident()
            .withf(move |id, _payload| id == existing_id_clone2)
            .times(1)
            .returning(|_, _| Ok(()));
        mock_incident.expect_create_incident().never();


        let mut monitor = instatus_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            threshold_seconds,
            30, // interval
        );

        let result = monitor.initialize().await;
        assert!(result.is_ok());
        assert!(monitor.base.active_incidents.is_empty(), "Existing incident should be resolved");
    }

    #[tokio::test]
    async fn test_instatusmonitor_initialize_existing_incident_remains_if_unhealthy() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "instatus_init_existing_unhealthy".to_string();
        let threshold_seconds = 120;
        let existing_incident_id = "existing_l2_incident_unhealthy".to_string();

        // 1. base.check_existing_incidents finds an incident
        let comp_id_clone = component_id.clone();
        let existing_id_clone1 = existing_incident_id.clone();
        mock_incident.expect_open_incident()
            .withf(move |id: &str| id == comp_id_clone)
            .times(1)
            .returning(move |_| Ok(Some(existing_id_clone1.clone())));

        // 2. check_initial_health calls CH, gets unhealthy L2 time
        let unhealthy_l2_time = time_secs_ago(threshold_seconds + 60); // Older than threshold
        mock_ch.expect_get_last_l2_head_time()
            .times(1)
            .returning(move || Ok(Some(unhealthy_l2_time)));

        // 3. handle (called by check_initial_health) should not resolve or create
        mock_incident.expect_resolve_incident().never();
        mock_incident.expect_create_incident().never();


        let mut monitor = instatus_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            threshold_seconds,
            30, // interval
        );

        let result = monitor.initialize().await;
        assert!(result.is_ok());
        assert_eq!(monitor.base.active_incidents.len(), 1, "Incident should remain");
        assert_eq!(monitor.base.active_incidents.get(&()), Some(&existing_incident_id));
    }

    #[tokio::test]
    async fn test_instatusmonitor_check_initial_health_active_incident_resolves_if_healthy() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "instatus_check_init_healthy_resolve".to_string();
        let threshold_seconds = 120;
        let existing_incident_id = "existing_l2_for_check_init_resolve".to_string();

        // 1. check_initial_health calls CH, gets healthy L2 time
        let healthy_l2_time = time_secs_ago(threshold_seconds - 45); // Younger than threshold
        mock_ch.expect_get_last_l2_head_time()
            .times(1)
            .returning(move || Ok(Some(healthy_l2_time)));

        // 2. handle (called by check_initial_health) resolves the incident
        let existing_id_clone = existing_incident_id.clone();
        mock_incident.expect_resolve_incident()
            .withf(move |id, _payload| id == existing_id_clone)
            .times(1)
            .returning(|_, _| Ok(()));
        mock_incident.expect_create_incident().never();


        let mut monitor = instatus_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            threshold_seconds,
            30, // interval
        );
        // Manually set active incident
        monitor.base.active_incidents.insert((), existing_incident_id.clone());

        let result = monitor.check_initial_health().await;
        assert!(result.is_ok());
        assert!(monitor.base.active_incidents.is_empty(), "Existing incident should be resolved by check_initial_health");
    }

    #[tokio::test]
    async fn test_instatusmonitor_check_initial_health_active_incident_remains_if_unhealthy() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "instatus_check_init_unhealthy_remain".to_string();
        let threshold_seconds = 120;
        let existing_incident_id = "existing_l2_for_check_init_unhealthy".to_string();

        // 1. check_initial_health calls CH, gets unhealthy L2 time
        let unhealthy_l2_time = time_secs_ago(threshold_seconds + 75); // Older than threshold
        mock_ch.expect_get_last_l2_head_time()
            .times(1)
            .returning(move || Ok(Some(unhealthy_l2_time)));

        // 2. handle (called by check_initial_health) should not resolve or create
        mock_incident.expect_resolve_incident().never();
        mock_incident.expect_create_incident().never();


        let mut monitor = instatus_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            threshold_seconds,
            30, // interval
        );
        monitor.base.active_incidents.insert((), existing_incident_id.clone());

        let result = monitor.check_initial_health().await;
        assert!(result.is_ok());
        assert_eq!(monitor.base.active_incidents.len(), 1, "Incident should remain");
        assert_eq!(monitor.base.active_incidents.get(&()), Some(&existing_incident_id));
    }

    // --- Tests for Monitor trait methods on InstatusMonitor ---

    #[tokio::test]
    async fn test_instatusmonitor_trait_create_incident() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident_client = MockIncidentClient::new();
        let component_id = "instatus_trait_create_test".to_string();
        let threshold_seconds = 120; // Matches InstatusMonitor's threshold use in open()
        let expected_incident_id = "trait_l2_created_incident_1".to_string();

        let expected_id_clone = expected_incident_id.clone();
        mock_incident_client.expect_create_incident()
            .withf(move |p: &NewIncident| {
                p.name == "No L2 head events - Possible Outage" &&
                p.message.contains(&format!("No L2 head event for {}s", threshold_seconds))
            })
            .times(1)
            .returning(move |_| Ok(expected_id_clone.clone()));

        let monitor = instatus_monitor_with_mocks(
            mock_ch,
            mock_incident_client,
            component_id,
            threshold_seconds,
            30, // interval
        );

        // The key for InstatusMonitor is ()
        let result = monitor.create_incident(&()).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_incident_id);
    }

    #[tokio::test]
    async fn test_instatusmonitor_trait_resolve_incident() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident_client = MockIncidentClient::new();
        let component_id = "instatus_trait_resolve_test".to_string();
        let incident_id_to_resolve = "trait_l2_resolving_incident_2".to_string();

        let id_clone = incident_id_to_resolve.clone();
        mock_incident_client.expect_resolve_incident()
            .withf(move |id: &str, p: &ResolveIncident| {
                id == id_clone && p.status == IncidentState::Resolved
            })
            .times(1)
            .returning(|_, _| Ok(()));

        let monitor = instatus_monitor_with_mocks(
            mock_ch,
            mock_incident_client,
            component_id,
            120, // threshold
            30, // interval
        );

        let result = monitor.resolve_incident(&incident_id_to_resolve).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_instatusmonitor_trait_check_health_opens_incident_on_unhealthy() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident_client = MockIncidentClient::new();
        let component_id = "instatus_trait_check_health_open".to_string();
        let threshold_seconds = 120;
        let expected_incident_id = "check_health_l2_opened_3".to_string();

        // 1. CH returns L2 head time as unhealthy
        let unhealthy_l2_time = time_secs_ago(threshold_seconds + 60); // Older than threshold
        mock_ch.expect_get_last_l2_head_time()
            .times(1)
            .returning(move || Ok(Some(unhealthy_l2_time)));

        // 2. Handle (called by check_l2_head) should open an incident
        let expected_id_clone = expected_incident_id.clone();
        mock_incident_client.expect_create_incident()
             .withf(move |p: &NewIncident| { // Verify payload details specific to InstatusMonitor::open
                p.name == "No L2 head events - Possible Outage" &&
                p.message.contains(&format!("No L2 head event for {}s", threshold_seconds))
            })
            .times(1)
            .returning(move |_| Ok(expected_id_clone.clone()));


        let mut monitor = instatus_monitor_with_mocks(
            mock_ch,
            mock_incident_client,
            component_id,
            threshold_seconds,
            30, // interval
        );

        let result = monitor.check_health().await; // This calls check_l2_head -> handle
        assert!(result.is_ok());
        assert_eq!(monitor.base.active_incidents.len(), 1, "Incident should be opened by check_health");
        assert_eq!(monitor.base.active_incidents.get(&()), Some(&expected_incident_id));
    }

    // --- Tests for BatchProofTimeoutMonitor ---

    #[tokio::test]
    async fn test_check_unproven_batches_scenario1_new_overdue_batches_open_incidents() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "proof_timeout_s1".to_string();
        let proof_timeout_hours = 3;
        let proof_timeout_duration = Duration::from_secs(proof_timeout_hours * 60 * 60);

        let now = Utc::now();
        let overdue_batch_1_posted_at = now - ChronoDuration::from_std(proof_timeout_duration).unwrap() - ChronoDuration::hours(1);
        let overdue_batch_2_posted_at = now - ChronoDuration::from_std(proof_timeout_duration).unwrap() - ChronoDuration::hours(2);

        let unproven_batches_from_ch = vec![
            (1001u64, 1u64, overdue_batch_1_posted_at), // (l1_block_number, batch_id, posted_at)
            (1002u64, 2u64, overdue_batch_2_posted_at),
        ];

        // CH: get_unproved_batches_older_than returns two overdue batches
        let batches_clone = unproven_batches_from_ch.clone();
        mock_ch.expect_get_unproved_batches_older_than()
            .times(1)
            // .withf(move |cutoff| { (*cutoff < overdue_batch_1_posted_at) && (*cutoff < overdue_batch_2_posted_at)}) // Ensure cutoff is correctly calculated
            .returning(move |_cutoff| Ok(batches_clone.clone()));

        // CH: get_proved_batch_ids (called for each active incident, none initially, then for the two new ones if they were added and checked in same run)
        // For this scenario, assume no pre-existing active incidents.
        // If check_unproven_batches re-checks status of newly created incidents in the same run,
        // then we need to account for those calls. The current code iterates a snapshot.
        // For simplicity here, assume it won't immediately re-check proof status of just-created incidents.
        // If it does, then proved_batch_ids might be called for batch 1 and 2.
        // The current code does: 1. get unproven. 2. create for new. 3. iterate active. 4. check proof for active.
        // So, it *will* call get_proved_batch_ids for the newly created ones.
        mock_ch.expect_get_proved_batch_ids()
            .times(1) // It's called once with a list of active incidents.
            .returning(|| Ok(vec![])); // Assume they are not yet proven

        // IncidentClient: create_incident should be called for each new overdue batch
        mock_incident.expect_create_incident()
            .withf(move |p: &NewIncident| p.message.contains("Batch #1 has been waiting for proof"))
            .times(1)
            .returning(|_p| Ok("incident_id_1".to_string()));
        mock_incident.expect_create_incident()
            .withf(move |p: &NewIncident| p.message.contains("Batch #2 has been waiting for proof"))
            .times(1)
            .returning(|_p| Ok("incident_id_2".to_string()));


        let mut monitor = batch_proof_timeout_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            proof_timeout_hours,
            60, // interval
        );

        let result = monitor.check_unproven_batches().await;
        assert!(result.is_ok());
        assert_eq!(monitor.base.active_incidents.len(), 2, "Two incidents should be active");
        assert!(monitor.base.active_incidents.contains_key(&(1001, 1)));
        assert!(monitor.base.active_incidents.contains_key(&(1002, 2)));
    }

    #[tokio::test]
    async fn test_check_unproven_batches_scenario2_active_incidents_some_proven_resolve() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "proof_timeout_s2".to_string();
        let proof_timeout_hours = 3;

        let active_incident_key_1 = (2001u64, 10u64); // (l1_block_number, batch_id) - Proven
        let active_incident_id_1 = "incident_id_10_proven".to_string();
        let active_incident_key_2 = (2002u64, 20u64); // Still unproven
        let active_incident_id_2 = "incident_id_20_unproven".to_string();

        // CH: get_unproved_batches_older_than returns no new overdue batches
        mock_ch.expect_get_unproved_batches_older_than()
            .times(1)
            .returning(|_cutoff| Ok(vec![]));

        // CH: get_proved_batch_ids returns that batch 10 is proven
        mock_ch.expect_get_proved_batch_ids()
            .times(1)
            .returning(|| Ok(vec![10u64])); // Batch 10 is proven

        // IncidentClient: resolve_incident should be called for batch 10
        let id_1_clone = active_incident_id_1.clone();
        mock_incident.expect_resolve_incident()
            .withf(move |id, _payload| id == id_1_clone)
            .times(1)
            .returning(|_, _| Ok(()));
        // No call to create_incident
        mock_incident.expect_create_incident().never();


        let mut monitor = batch_proof_timeout_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            proof_timeout_hours,
            60, // interval
        );

        // Setup active incidents
        monitor.base.active_incidents.insert(active_incident_key_1, active_incident_id_1.clone());
        monitor.base.active_incidents.insert(active_incident_key_2, active_incident_id_2.clone());

        let result = monitor.check_unproven_batches().await;
        assert!(result.is_ok());

        assert_eq!(monitor.base.active_incidents.len(), 1, "One incident should remain active");
        assert!(!monitor.base.active_incidents.contains_key(&active_incident_key_1), "Proven incident should be removed");
        assert!(monitor.base.active_incidents.contains_key(&active_incident_key_2), "Unproven incident should remain");
        assert_eq!(monitor.base.active_incidents.get(&active_incident_key_2), Some(&active_incident_id_2));
    }

    #[tokio::test]
    async fn test_check_unproven_batches_scenario3_new_and_existing_overdue_batches() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "proof_timeout_s3".to_string();
        let proof_timeout_hours = 3;
        let proof_timeout_duration = Duration::from_secs(proof_timeout_hours * 60 * 60);

        let now = Utc::now();
        // Batch 30: Overdue, already has an active incident
        let batch_30_posted_at = now - ChronoDuration::from_std(proof_timeout_duration).unwrap() - ChronoDuration::hours(1);
        let existing_incident_key_30 = (3001u64, 30u64);
        let existing_incident_id_30 = "incident_id_30_existing".to_string();

        // Batch 40: Overdue, new, should trigger incident creation
        let batch_40_posted_at = now - ChronoDuration::from_std(proof_timeout_duration).unwrap() - ChronoDuration::hours(2);

        let unproven_batches_from_ch = vec![
            (existing_incident_key_30.0, existing_incident_key_30.1, batch_30_posted_at),
            (4001u64, 40u64, batch_40_posted_at),
        ];

        // CH: get_unproved_batches_older_than returns these two
        let batches_clone = unproven_batches_from_ch.clone();
        mock_ch.expect_get_unproved_batches_older_than()
            .times(1)
            .returning(move |_| Ok(batches_clone.clone()));

        // CH: get_proved_batch_ids for active incidents (batch 30 and newly created 40)
        // Assume neither are proven yet for this scenario.
        mock_ch.expect_get_proved_batch_ids()
            .times(1)
            .returning(|| Ok(vec![])); // Neither 30 nor 40 are proven

        // IncidentClient: create_incident should be called only for batch 40
        mock_incident.expect_create_incident()
            .withf(move |p: &NewIncident| p.message.contains("Batch #40 has been waiting for proof"))
            .times(1)
            .returning(|_p| Ok("incident_id_40_new".to_string()));
        // No create_incident for batch 30 (already active)
        mock_incident.expect_create_incident()
            .withf(move |p: &NewIncident| p.message.contains("Batch #30"))
            .never();
        // No resolve_incident calls
        mock_incident.expect_resolve_incident().never();


        let mut monitor = batch_proof_timeout_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            proof_timeout_hours,
            60, // interval
        );

        // Setup existing active incident for batch 30
        monitor.base.active_incidents.insert(existing_incident_key_30, existing_incident_id_30.clone());

        let result = monitor.check_unproven_batches().await;
        assert!(result.is_ok());

        assert_eq!(monitor.base.active_incidents.len(), 2, "Two incidents should be active");
        // Existing incident for batch 30 should remain
        assert_eq!(monitor.base.active_incidents.get(&existing_incident_key_30), Some(&existing_incident_id_30));
        // New incident for batch 40 should be added
        assert!(monitor.base.active_incidents.contains_key(&(4001, 40)));
        assert_eq!(monitor.base.active_incidents.get(&(4001, 40)), Some(&"incident_id_40_new".to_string()));
    }

    #[tokio::test]
    async fn test_check_unproven_batches_scenario4_catch_all_incident_resolution() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "proof_timeout_s4_catch_all".to_string();
        let proof_timeout_hours = 3;

        let catch_all_key = (0u64, 0u64);
        let catch_all_incident_id = "incident_id_catch_all".to_string();

        // CH: get_unproved_batches_older_than returns no new overdue batches
        mock_ch.expect_get_unproved_batches_older_than()
            .times(1)
            .returning(|_cutoff| Ok(vec![]));

        // CH: get_proved_batch_ids might be called if there were other active incidents.
        // Since we only have the catch-all, and its batch_id is 0, it's skipped in the loop.
        // So, get_proved_batch_ids might not be called, or if called with an empty list of specific batches to check, it's fine.
        // For this specific path, it's called once before the loop for active_incidents_snapshot.
        // The loop `for (key, incident_id) in active_incidents_snapshot` will see the catch-all,
        // batch_id is 0, so it continues. Then the special case check happens.
        mock_ch.expect_get_proved_batch_ids().times(1).returning(|| Ok(vec![]));


        // IncidentClient: resolve_incident should be called for the catch-all incident
        let id_catch_all_clone = catch_all_incident_id.clone();
        mock_incident.expect_resolve_incident()
            .withf(move |id, _payload| id == id_catch_all_clone)
            .times(1)
            .returning(|_, _| Ok(()));
        mock_incident.expect_create_incident().never();


        let mut monitor = batch_proof_timeout_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            proof_timeout_hours,
            60, // interval
        );

        // Setup catch-all active incident. This is the *only* active incident.
        monitor.base.active_incidents.insert(catch_all_key, catch_all_incident_id.clone());
        assert_eq!(monitor.base.active_incidents.len(), 1, "Pre-condition: Only catch-all incident is active");


        let result = monitor.check_unproven_batches().await;
        assert!(result.is_ok());

        assert!(monitor.base.active_incidents.is_empty(), "Catch-all incident should be resolved and removed");
    }

    #[tokio::test]
    async fn test_batch_proof_timeout_monitor_initialize_finds_catch_all() {
        let mut mock_ch = MockClickhouseClient::new(); // Not directly used by initialize's primary path
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "proof_timeout_init_catch_all".to_string();
        let proof_timeout_hours = 3;
        let catch_all_incident_id = "init_catch_all_id_found".to_string();
        let catch_all_key = (0u64, 0u64);

        // IncidentClient: open_incident (called by base.check_existing_incidents) finds the catch-all.
        let comp_id_clone = component_id.clone();
        let id_clone = catch_all_incident_id.clone();
        mock_incident.expect_open_incident()
            .withf(move |cid: &str| cid == comp_id_clone) // check_existing_incidents calls with component_id
            .times(1)
            .returning(move |_| Ok(Some(id_clone.clone())));


        let mut monitor = batch_proof_timeout_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            proof_timeout_hours,
            60, // interval
        );

        let result = monitor.initialize().await;
        assert!(result.is_ok());

        assert_eq!(monitor.base.active_incidents.len(), 1, "Catch-all incident should be active");
        assert!(monitor.base.active_incidents.contains_key(&catch_all_key));
        assert_eq!(monitor.base.active_incidents.get(&catch_all_key), Some(&catch_all_incident_id));
    }

    // --- Tests for Monitor trait methods on BatchProofTimeoutMonitor ---

    #[tokio::test]
    async fn test_batch_proof_timeout_monitor_trait_create_incident() {
        let mut mock_ch = MockClickhouseClient::new(); // Not directly used by this path of create_incident
        let mut mock_incident_client = MockIncidentClient::new();
        let component_id = "proof_timeout_trait_create".to_string();
        let proof_timeout_hours = 3;
        let expected_incident_id = "trait_proof_timeout_created_1".to_string();
        let batch_key_to_create = (5001u64, 50u64); // (l1_block_number, batch_id)

        let expected_id_clone = expected_incident_id.clone();
        mock_incident_client.expect_create_incident()
            .withf(move |p: &NewIncident| {
                p.name == format!("Batch #{} Not Proven - Timeout", batch_key_to_create.1) &&
                // age_hours is 0 in this direct call via trait
                p.message == format!("Batch #{} has been waiting for proof for 0h (threshold: {}h)", batch_key_to_create.1, proof_timeout_hours)
            })
            .times(1)
            .returning(move |_| Ok(expected_id_clone.clone()));

        let monitor = batch_proof_timeout_monitor_with_mocks(
            mock_ch,
            mock_incident_client,
            component_id,
            proof_timeout_hours,
            60, // interval
        );

        let result = monitor.create_incident(&batch_key_to_create).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_incident_id);
    }

    #[tokio::test]
    async fn test_batch_proof_timeout_monitor_trait_resolve_incident() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident_client = MockIncidentClient::new();
        let component_id = "proof_timeout_trait_resolve".to_string();
        let incident_id_to_resolve = "trait_proof_timeout_resolving_2".to_string();

        let id_clone = incident_id_to_resolve.clone();
        mock_incident_client.expect_resolve_incident()
            .withf(move |id: &str, p: &ResolveIncident| {
                id == id_clone && p.status == IncidentState::Resolved
            })
            .times(1)
            .returning(|_, _| Ok(()));

        let monitor = batch_proof_timeout_monitor_with_mocks(
            mock_ch,
            mock_incident_client,
            component_id,
            3, // proof_timeout_hours
            60, // interval
        );

        let result = monitor.resolve_incident(&incident_id_to_resolve).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_batch_proof_timeout_monitor_trait_check_health_opens_incident() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "proof_timeout_trait_check_health".to_string();
        let proof_timeout_hours = 3;
        let proof_timeout_duration = Duration::from_secs(proof_timeout_hours * 60 * 60);
        let expected_incident_id = "trait_check_health_proof_timeout_opened".to_string();

        let now = Utc::now();
        let overdue_batch_posted_at = now - ChronoDuration::from_std(proof_timeout_duration).unwrap() - ChronoDuration::hours(1);
        let unproven_batch_from_ch = vec![(6001u64, 60u64, overdue_batch_posted_at)]; // (l1, batch_id, posted_at)

        // CH: get_unproved_batches_older_than returns one overdue batch
        let batch_clone = unproven_batch_from_ch.clone();
        mock_ch.expect_get_unproved_batches_older_than()
            .times(1)
            .returning(move |_| Ok(batch_clone.clone()));

        // CH: get_proved_batch_ids for the (now active) incident for batch 60
        mock_ch.expect_get_proved_batch_ids()
            .times(1)
            .returning(|| Ok(vec![])); // Batch 60 is not proven

        // IncidentClient: create_incident for batch 60
        let expected_id_clone = expected_incident_id.clone();
        mock_incident.expect_create_incident()
            .withf(move |p: &NewIncident| p.message.contains("Batch #60 has been waiting for proof"))
            .times(1)
            .returning(move |_| Ok(expected_id_clone.clone()));


        let mut monitor = batch_proof_timeout_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            proof_timeout_hours,
            60, // interval
        );

        let result = monitor.check_health().await; // This calls check_unproven_batches
        assert!(result.is_ok());
        assert_eq!(monitor.base.active_incidents.len(), 1, "Incident should be opened by check_health");
        assert!(monitor.base.active_incidents.contains_key(&(6001, 60)));
        assert_eq!(monitor.base.active_incidents.get(&(6001, 60)), Some(&expected_incident_id));
    }

    // --- Tests for BatchVerifyTimeoutMonitor ---

    #[tokio::test]
    async fn test_check_unverified_batches_scenario1_new_overdue_batches_open_incidents() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "verify_timeout_s1".to_string();
        let verify_timeout_hours = 1; // Using a smaller timeout for verify
        let verify_timeout_duration = Duration::from_secs(verify_timeout_hours * 60 * 60);

        let now = Utc::now();
        // Note: BatchVerifyTimeoutMonitor uses just batch_id as key
        let overdue_batch_70_posted_at = now - ChronoDuration::from_std(verify_timeout_duration).unwrap() - ChronoDuration::hours(1);
        let overdue_batch_80_posted_at = now - ChronoDuration::from_std(verify_timeout_duration).unwrap() - ChronoDuration::hours(2);

        // CH returns these batches as unverified and older than cutoff
        let unverified_batches_from_ch = vec![
            (7001u64, 70u64, overdue_batch_70_posted_at), // (l1_block_number, batch_id, posted_at)
            (8001u64, 80u64, overdue_batch_80_posted_at),
        ];
        let batches_clone = unverified_batches_from_ch.clone();
        mock_ch.expect_get_unverified_batches_older_than()
            .times(1)
            .returning(move |_cutoff| Ok(batches_clone.clone()));

        // CH: get_verified_batch_ids (called for active incidents)
        // Since these are new, they won't be in active_incidents yet for the first part of the function.
        // The loop for resolving active incidents will run on a snapshot.
        // If new incidents are added and then immediately checked, this mock needs to reflect they are not verified.
        mock_ch.expect_get_verified_batch_ids()
            .times(1) // Called once when iterating active incidents (which are now 70 and 80)
            .returning(|| Ok(vec![])); // Neither 70 nor 80 are verified yet

        // IncidentClient: create_incident for 70 and 80
        mock_incident.expect_create_incident()
            .withf(move |p: &NewIncident| p.message.contains("Batch #70 has been waiting for verification"))
            .times(1)
            .returning(|_p| Ok("incident_id_70".to_string()));
        mock_incident.expect_create_incident()
            .withf(move |p: &NewIncident| p.message.contains("Batch #80 has been waiting for verification"))
            .times(1)
            .returning(|_p| Ok("incident_id_80".to_string()));


        let mut monitor = batch_verify_timeout_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            verify_timeout_hours,
            60, // interval
        );

        let result = monitor.check_unverified_batches().await;
        assert!(result.is_ok());
        assert_eq!(monitor.base.active_incidents.len(), 2, "Two incidents should be active");
        assert!(monitor.base.active_incidents.contains_key(&70u64));
        assert!(monitor.base.active_incidents.contains_key(&80u64));
    }

    #[tokio::test]
    async fn test_check_unverified_batches_scenario2_active_incidents_some_verified_resolve() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "verify_timeout_s2".to_string();
        let verify_timeout_hours = 1;

        let active_incident_key_70 = 70u64; // Verified
        let active_incident_id_70 = "incident_id_70_verified".to_string();
        let active_incident_key_80 = 80u64; // Still unverified
        let active_incident_id_80 = "incident_id_80_unverified".to_string();

        // CH: get_unverified_batches_older_than returns no new overdue batches
        mock_ch.expect_get_unverified_batches_older_than()
            .times(1)
            .returning(|_cutoff| Ok(vec![]));

        // CH: get_verified_batch_ids returns that batch 70 is verified
        mock_ch.expect_get_verified_batch_ids()
            .times(1)
            .returning(|| Ok(vec![70u64])); // Batch 70 is verified

        // IncidentClient: resolve_incident should be called for batch 70
        let id_70_clone = active_incident_id_70.clone();
        mock_incident.expect_resolve_incident()
            .withf(move |id, _payload| id == id_70_clone)
            .times(1)
            .returning(|_, _| Ok(()));
        mock_incident.expect_create_incident().never();


        let mut monitor = batch_verify_timeout_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            verify_timeout_hours,
            60, // interval
        );

        // Setup active incidents
        monitor.base.active_incidents.insert(active_incident_key_70, active_incident_id_70.clone());
        monitor.base.active_incidents.insert(active_incident_key_80, active_incident_id_80.clone());

        let result = monitor.check_unverified_batches().await;
        assert!(result.is_ok());

        assert_eq!(monitor.base.active_incidents.len(), 1, "One incident should remain active");
        assert!(!monitor.base.active_incidents.contains_key(&active_incident_key_70), "Verified incident should be removed");
        assert!(monitor.base.active_incidents.contains_key(&active_incident_key_80), "Unverified incident should remain");
        assert_eq!(monitor.base.active_incidents.get(&active_incident_key_80), Some(&active_incident_id_80));
    }

    #[tokio::test]
    async fn test_check_unverified_batches_scenario3_new_and_existing_overdue_batches() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "verify_timeout_s3".to_string();
        let verify_timeout_hours = 1;
        let verify_timeout_duration = Duration::from_secs(verify_timeout_hours * 60 * 60);

        let now = Utc::now();
        // Batch 90: Overdue, already has an active incident
        let batch_90_posted_at = now - ChronoDuration::from_std(verify_timeout_duration).unwrap() - ChronoDuration::hours(1);
        let existing_incident_key_90 = 90u64;
        let existing_incident_id_90 = "incident_id_90_existing".to_string();

        // Batch 100: Overdue, new, should trigger incident creation
        let batch_100_posted_at = now - ChronoDuration::from_std(verify_timeout_duration).unwrap() - ChronoDuration::hours(2);

        let unverified_batches_from_ch = vec![
            (9001u64, existing_incident_key_90, batch_90_posted_at), // (l1_block_number, batch_id, posted_at)
            (10001u64, 100u64, batch_100_posted_at),
        ];

        // CH: get_unverified_batches_older_than returns these two
        let batches_clone = unverified_batches_from_ch.clone();
        mock_ch.expect_get_unverified_batches_older_than()
            .times(1)
            .returning(move |_| Ok(batches_clone.clone()));

        // CH: get_verified_batch_ids for active incidents (batch 90 and newly created 100)
        // Assume neither are verified yet for this scenario.
        mock_ch.expect_get_verified_batch_ids()
            .times(1)
            .returning(|| Ok(vec![])); // Neither 90 nor 100 are verified

        // IncidentClient: create_incident should be called only for batch 100
        mock_incident.expect_create_incident()
            .withf(move |p: &NewIncident| p.message.contains("Batch #100 has been waiting for verification"))
            .times(1)
            .returning(|_p| Ok("incident_id_100_new".to_string()));
        // No create_incident for batch 90 (already active)
        mock_incident.expect_create_incident()
            .withf(move |p: &NewIncident| p.message.contains("Batch #90"))
            .never();
        mock_incident.expect_resolve_incident().never();


        let mut monitor = batch_verify_timeout_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            verify_timeout_hours,
            60, // interval
        );

        // Setup existing active incident for batch 90
        monitor.base.active_incidents.insert(existing_incident_key_90, existing_incident_id_90.clone());

        let result = monitor.check_unverified_batches().await;
        assert!(result.is_ok());

        assert_eq!(monitor.base.active_incidents.len(), 2, "Two incidents should be active");
        assert_eq!(monitor.base.active_incidents.get(&existing_incident_key_90), Some(&existing_incident_id_90));
        assert!(monitor.base.active_incidents.contains_key(&100u64));
        assert_eq!(monitor.base.active_incidents.get(&100u64), Some(&"incident_id_100_new".to_string()));
    }

    #[tokio::test]
    async fn test_check_unverified_batches_scenario4_catch_all_incident_resolution() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "verify_timeout_s4_catch_all".to_string();
        let verify_timeout_hours = 1;

        let catch_all_key = 0u64;
        let catch_all_incident_id = "incident_id_verify_catch_all".to_string();

        // CH: get_unverified_batches_older_than returns no new overdue batches
        mock_ch.expect_get_unverified_batches_older_than()
            .times(1)
            .returning(|_cutoff| Ok(vec![])); // Crucial: no specific unverified batches

        // CH: get_verified_batch_ids. The loop for specific batches won't run if active_incidents only has catch-all.
        // If it were called (e.g. if the snapshot was taken earlier), it should return empty or irrelevant.
        // The current code takes snapshot, then iterates. The batch_id == 0 check prevents call for catch-all.
        mock_ch.expect_get_verified_batch_ids().times(1).returning(|| Ok(vec![]));


        // IncidentClient: resolve_incident should be called for the catch-all incident
        let id_catch_all_clone = catch_all_incident_id.clone();
        mock_incident.expect_resolve_incident()
            .withf(move |id, _payload| id == id_catch_all_clone)
            .times(1)
            .returning(|_, _| Ok(()));
        mock_incident.expect_create_incident().never();


        let mut monitor = batch_verify_timeout_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            verify_timeout_hours,
            60, // interval
        );

        // Setup catch-all active incident. This is the *only* active incident.
        monitor.base.active_incidents.insert(catch_all_key, catch_all_incident_id.clone());
        assert_eq!(monitor.base.active_incidents.len(), 1, "Pre-condition: Only catch-all incident is active");

        let result = monitor.check_unverified_batches().await;
        assert!(result.is_ok());

        assert!(monitor.base.active_incidents.is_empty(), "Catch-all verify incident should be resolved and removed");
    }

    #[tokio::test]
    async fn test_batch_verify_timeout_monitor_initialize_finds_catch_all() {
        let mut mock_ch = MockClickhouseClient::new(); // Not directly used by initialize's primary path
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "verify_timeout_init_catch_all".to_string();
        let verify_timeout_hours = 1;
        let catch_all_incident_id = "init_verify_catch_all_id_found".to_string();
        let catch_all_key = 0u64; // Key for BatchVerifyTimeoutMonitor's catch-all

        // IncidentClient: open_incident (called by base.check_existing_incidents) finds the catch-all.
        let comp_id_clone = component_id.clone();
        let id_clone = catch_all_incident_id.clone();
        mock_incident.expect_open_incident()
            .withf(move |cid: &str| cid == comp_id_clone) // check_existing_incidents calls with component_id
            .times(1)
            .returning(move |_| Ok(Some(id_clone.clone())));


        let mut monitor = batch_verify_timeout_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            verify_timeout_hours,
            60, // interval
        );

        let result = monitor.initialize().await;
        assert!(result.is_ok());

        assert_eq!(monitor.base.active_incidents.len(), 1, "Catch-all incident should be active");
        assert!(monitor.base.active_incidents.contains_key(&catch_all_key));
        assert_eq!(monitor.base.active_incidents.get(&catch_all_key), Some(&catch_all_incident_id));
    }

    // --- Tests for Monitor trait methods on BatchVerifyTimeoutMonitor ---

    #[tokio::test]
    async fn test_batch_verify_timeout_monitor_trait_create_incident() {
        let mut mock_ch = MockClickhouseClient::new(); // Not directly used by this path
        let mut mock_incident_client = MockIncidentClient::new();
        let component_id = "verify_timeout_trait_create".to_string();
        let verify_timeout_hours = 1;
        let expected_incident_id = "trait_verify_timeout_created_1".to_string();
        let batch_id_to_create = 110u64;

        let expected_id_clone = expected_incident_id.clone();
        mock_incident_client.expect_create_incident()
            .withf(move |p: &NewIncident| {
                p.name == format!("Batch #{} Not Verified - Timeout", batch_id_to_create) &&
                p.message == format!("Batch #{} has been waiting for verification for over 0h (threshold: {}h)", batch_id_to_create, verify_timeout_hours)
            })
            .times(1)
            .returning(move |_| Ok(expected_id_clone.clone()));

        let monitor = batch_verify_timeout_monitor_with_mocks(
            mock_ch,
            mock_incident_client,
            component_id,
            verify_timeout_hours,
            60, // interval
        );

        let result = monitor.create_incident(&batch_id_to_create).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_incident_id);
    }

    #[tokio::test]
    async fn test_batch_verify_timeout_monitor_trait_resolve_incident() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident_client = MockIncidentClient::new();
        let component_id = "verify_timeout_trait_resolve".to_string();
        let incident_id_to_resolve = "trait_verify_timeout_resolving_2".to_string();

        let id_clone = incident_id_to_resolve.clone();
        mock_incident_client.expect_resolve_incident()
            .withf(move |id: &str, p: &ResolveIncident| {
                id == id_clone && p.status == IncidentState::Resolved
            })
            .times(1)
            .returning(|_, _| Ok(()));

        let monitor = batch_verify_timeout_monitor_with_mocks(
            mock_ch,
            mock_incident_client,
            component_id,
            1, // verify_timeout_hours
            60, // interval
        );

        let result = monitor.resolve_incident(&incident_id_to_resolve).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_batch_verify_timeout_monitor_trait_check_health_opens_incident() {
        let mut mock_ch = MockClickhouseClient::new();
        let mut mock_incident = MockIncidentClient::new();
        let component_id = "verify_timeout_trait_check_health".to_string();
        let verify_timeout_hours = 1;
        let verify_timeout_duration = Duration::from_secs(verify_timeout_hours * 60 * 60);
        let expected_incident_id = "trait_check_health_verify_timeout_opened".to_string();

        let now = Utc::now();
        let overdue_batch_posted_at = now - ChronoDuration::from_std(verify_timeout_duration).unwrap() - ChronoDuration::hours(1);
        // (l1_block_number, batch_id, posted_at) - l1_block_number isn't used by BatchVerifyTimeoutMonitor's key directly
        let unverified_batch_from_ch = vec![(12001u64, 120u64, overdue_batch_posted_at)];

        // CH: get_unverified_batches_older_than returns one overdue batch
        let batch_clone = unverified_batch_from_ch.clone();
        mock_ch.expect_get_unverified_batches_older_than()
            .times(1)
            .returning(move |_| Ok(batch_clone.clone()));

        // CH: get_verified_batch_ids for the (now active) incident for batch 120
        mock_ch.expect_get_verified_batch_ids()
            .times(1)
            .returning(|| Ok(vec![])); // Batch 120 is not verified

        // IncidentClient: create_incident for batch 120
        let expected_id_clone = expected_incident_id.clone();
        mock_incident.expect_create_incident()
            .withf(move |p: &NewIncident| p.message.contains("Batch #120 has been waiting for verification"))
            .times(1)
            .returning(move |_| Ok(expected_id_clone.clone()));


        let mut monitor = batch_verify_timeout_monitor_with_mocks(
            mock_ch,
            mock_incident,
            component_id,
            verify_timeout_hours,
            60, // interval
        );

        let result = monitor.check_health().await; // This calls check_unverified_batches
        assert!(result.is_ok());
        assert_eq!(monitor.base.active_incidents.len(), 1, "Incident should be opened by check_health");
        assert!(monitor.base.active_incidents.contains_key(&120u64));
        assert_eq!(monitor.base.active_incidents.get(&120u64), Some(&expected_incident_id));
    }

    // Helper to create a ClickhouseClient for tests
    fn mock_clickhouse_client() -> (ActualClickhouseClient, ServerGuard) { // Renamed return type
        let server = mockito::Server::new();
        let url = Url::parse(&server.url()).unwrap();
        // This is ActualClickhouseClient, not the mock.
        // The mock_clickhouse_client name is now ambiguous.
        // Let's assume this is for other tests and keep it, but our new tests will use MockClickhouseClient.
        let client = ActualClickhouseClient::new(
            url,
            "test_db".to_string(),
            "user".to_string(),
            "pass".to_string(),
        )
        .unwrap();
        (client, server)
    }

    // Helper to create an IncidentClient for tests
    fn mock_incident_client() -> (ActualIncidentClient, ServerGuard) { // Renamed return type
        let server = mockito::Server::new();
        let url = Url::parse(&server.url()).unwrap();
        // This is ActualIncidentClient, not the mock.
        let client = ActualIncidentClient::with_base_url(
            "test_api_key".to_string(),
            "test_page_id".to_string(),
            url,
        );
        (client, server)
    }

    #[test]
    fn test_batch_proof_timeout_monitor_creation() {
        let (ch_client, _ch_server) = mock_clickhouse_client();
        let (incident_client, _incident_server) = mock_incident_client();

        let _monitor = BatchProofTimeoutMonitor::new(
            ch_client,
            incident_client,
            "component_proof_timeout".to_string(),
            Duration::from_secs(3 * 60 * 60), // 3 hours
            Duration::from_secs(60),          // 1 minute interval
        );
    }

    #[test]
    fn test_batch_verify_timeout_monitor_creation() {
        let (ch_client, _ch_server) = mock_clickhouse_client();
        let (incident_client, _incident_server) = mock_incident_client();

        let _monitor = BatchVerifyTimeoutMonitor::new(
            ch_client,
            incident_client,
            "component_verify_timeout".to_string(),
            Duration::from_secs(60 * 60), // 1 hour
            Duration::from_secs(60),      // 1 minute interval
        );
    }
}
