use crate::{
    base_monitor::{BaseMonitor, Monitor},
    client::Client as IncidentClient,
};
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
        healthy_needed: u8,
    ) -> Self {
        Self {
            base: BaseMonitor::new(clickhouse, client, component_id, interval, healthy_needed),
            threshold,
        }
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
            healthy_seen = self.base.healthy_seen,
            healthy_needed = self.base.healthy_needed,
            "Batch event status"
        );

        let has_active = !self.base.active_incidents.is_empty();

        match (has_active, batch_healthy, l2_healthy) {
            // Batch outage while L2 healthy: open incident
            (false, false, true) => {
                let id = self.open(last_batch).await?;
                self.base.active_incidents.insert((), id);
                self.base.healthy_seen = 0;
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

impl Monitor for InstatusL1Monitor {
    type IncidentKey = ();

    fn create_incident(
        &self,
        _key: &Self::IncidentKey,
        _data: &impl std::fmt::Debug,
    ) -> impl std::future::Future<Output = Result<String>> + Send {
        async move { self.open(Utc::now()).await }
    }

    fn resolve_incident(
        &self,
        incident_id: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send {
        async move {
            let payload = self.base.create_resolve_payload();
            self.base.resolve_incident_with_payload(incident_id, &payload).await
        }
    }

    fn check_health(&mut self) -> impl std::future::Future<Output = Result<()>> + Send {
        async move { self.check_batch_and_l2().await }
    }

    fn initialize(&mut self) -> impl std::future::Future<Output = Result<()>> + Send {
        async move {
            self.base.check_existing_incidents(()).await?;
            self.check_initial_health().await
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
        healthy_needed: u8,
    ) -> Self {
        Self {
            base: BaseMonitor::new(clickhouse, client, component_id, interval, healthy_needed),
            threshold,
        }
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
            healthy_seen = self.base.healthy_seen,
            healthy_needed = self.base.healthy_needed,
            "L2 head event status"
        );

        let has_active = !self.base.active_incidents.is_empty();

        match (has_active, is_healthy) {
            // outage begins
            (false, false) => {
                let id = self.open(last).await?;
                self.base.active_incidents.insert((), id);
                self.base.healthy_seen = 0;
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
        if let Some(_) = self.base.active_incidents.values().next() {
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

impl Monitor for InstatusMonitor {
    type IncidentKey = ();

    fn create_incident(
        &self,
        _key: &Self::IncidentKey,
        _data: &impl std::fmt::Debug,
    ) -> impl std::future::Future<Output = Result<String>> + Send {
        async move { self.open(Utc::now()).await }
    }

    fn resolve_incident(
        &self,
        incident_id: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send {
        async move {
            let payload = self.base.create_resolve_payload();
            self.base.resolve_incident_with_payload(incident_id, &payload).await
        }
    }

    fn check_health(&mut self) -> impl std::future::Future<Output = Result<()>> + Send {
        async move { self.check_l2_head().await }
    }

    fn initialize(&mut self) -> impl std::future::Future<Output = Result<()>> + Send {
        async move {
            self.base.check_existing_incidents(()).await?;
            self.check_initial_health().await
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
