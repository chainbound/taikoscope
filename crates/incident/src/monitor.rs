use crate::client::Client as IncidentClient;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use clickhouse::ClickhouseClient;
use eyre::Result;
use serde::Serialize;
use std::time::Duration;
use tokio::task::JoinHandle;
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

/// Monitors `ClickHouse` L2 head events and manages Instatus incidents.
/// Polls `ClickHouse` every `interval` seconds; if no L2 head event for `threshold` seconds, it
/// creates an incident; resolves when events resume.
#[derive(Debug)]
pub struct InstatusMonitor {
    clickhouse: ClickhouseClient,
    client: IncidentClient,
    component_id: String,
    threshold: Duration,
    interval: Duration,
    active: Option<String>,
    healthy_needed: u8,
    healthy_seen: u8,
}

impl InstatusMonitor {
    /// Creates a new `InstatusMonitor` with a 30s threshold and interval.
    pub const fn new(
        clickhouse: ClickhouseClient,
        client: IncidentClient,
        component_id: String,
        threshold: Duration,
        interval: Duration,
        active: Option<String>,
        healthy_needed: u8,
    ) -> Self {
        Self {
            clickhouse,
            client,
            component_id,
            threshold,
            interval,
            active,
            healthy_needed,
            healthy_seen: 0,
        }
    }

    /// Spawns the monitor on the Tokio runtime.
    pub fn spawn(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            if let Err(e) = self.run().await {
                error!(%e, "monitor exited unexpectedly");
            }
        })
    }

    /// Runs the monitor.
    async fn run(mut self) -> Result<()> {
        self.active = self.client.open_incident(&self.component_id).await?;
        let mut interval = tokio::time::interval(self.interval);

        loop {
            interval.tick().await;
            match self.clickhouse.get_last_l2_head_time().await {
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
        }
    }

    /// Handles a new L2 head event.
    async fn handle(&mut self, last: DateTime<Utc>) -> Result<()> {
        let age = Utc::now().signed_duration_since(last).to_std()?;
        let is_healthy = !age.gt(&self.threshold);

        debug!(
            active_incident = ?self.active,
            age_seconds = ?age.as_secs(),
            threshold_seconds = ?self.threshold.as_secs(),
            is_healthy = is_healthy,
            healthy_seen = self.healthy_seen,
            healthy_needed = self.healthy_needed,
            "L2 head event status"
        );

        match (&self.active, is_healthy) {
            // outage begins
            (None, false) => {
                self.active = Some(self.open(last).await?);
                self.healthy_seen = 0;
            }
            // still down
            (Some(_), false) => self.healthy_seen = 0,
            // up again
            (Some(id), true) => {
                self.healthy_seen += 1;
                if self.healthy_seen >= self.healthy_needed {
                    self.close(id).await?;
                    self.active = None;
                    self.healthy_seen = 0;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Opens a new incident.
    async fn open(&self, last: DateTime<Utc>) -> Result<String> {
        // The incident starts when the L2 block should have been processed
        let started = (last + ChronoDuration::seconds(2)).to_rfc3339();

        let body = NewIncident {
            name: "No L2 head events - Possible Outage".into(),
            message: format!("No L2 head event for {}s", self.threshold.as_secs()),
            status: IncidentState::Investigating,
            components: vec![self.component_id.clone()],
            statuses: vec![ComponentStatus::major_outage(&self.component_id)],
            notify: true,
            started: Some(started),
        };
        let id = self.client.create_incident(&body).await?;

        info!(
            incident_id = %id,
            name = %body.name,
            message = %body.message,
            status = ?body.status,
            components = ?body.components,
            statuses = ?body.statuses,
            notify = %body.notify,
            started = ?body.started,
            "Created incident"
        );
        Ok(id)
    }

    /// Closes an incident.
    async fn close(&self, id: &str) -> Result<()> {
        let body = ResolveIncident {
            status: IncidentState::Resolved,
            components: vec![self.component_id.clone()],
            statuses: vec![ComponentStatus::operational(&self.component_id)],
            notify: true,
            started: Some(Utc::now().to_rfc3339()),
        };

        debug!(%id, "Closing incident with body: {:?}", body);

        match self.client.resolve_incident(id, &body).await {
            Ok(_) => {
                info!(%id, "Successfully resolved incident");
                Ok(())
            }
            Err(e) => {
                error!(%id, error = %e, "Failed to resolve incident");
                Err(e)
            }
        }
    }
}
