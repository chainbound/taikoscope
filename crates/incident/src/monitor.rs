use crate::client::Client as IncidentClient;
use chrono::{DateTime, Utc};
use clickhouse::ClickhouseClient;
use eyre::Result;
use serde::Serialize;
use std::time::Duration;
use tracing::{error, info};

/// Incident‐level state sent to Instatus.
#[derive(Debug, Serialize, Clone, Copy)]
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
#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
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
    pub status: String,
}

impl ComponentStatus {
    /// Create a new component status for a major outage.
    pub fn major_outage(id: &str) -> Self {
        Self { id: id.into(), status: "MAJOROUTAGE".into() }
    }

    /// Create a new component status for an operational component.
    pub fn operational(id: &str) -> Self {
        Self { id: id.into(), status: "OPERATIONAL".into() }
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
    pub status: String,
    /// Affected component IDs
    pub components: Vec<String>,
    /// Component statuses
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
    /// Update message
    pub message: String,
    /// Status (should be RESOLVED)
    pub status: String,
    /// Affected component IDs
    pub components: Vec<String>,
    /// Component statuses
    pub statuses: Vec<ComponentStatus>,
    /// Whether to notify subscribers
    pub notify: bool,
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
    ) -> Self {
        Self { clickhouse, client, component_id, threshold, interval, active }
    }

    /// Spawns the monitor on the Tokio runtime.
    pub fn spawn(mut self) {
        tokio::spawn(async move {
            if let Err(e) = self.run().await {
                error!(%e, "Instatus monitor exited");
            }
        });
    }

    async fn run(&mut self) -> Result<()> {
        self.active = self.client.open_incident(&self.component_id).await?;
        let mut tick = tokio::time::interval(self.interval);

        loop {
            tick.tick().await;
            if let Some(last) = self.clickhouse.get_last_l2_head_time().await? {
                self.handle(last).await?;
            }
        }
    }

    async fn handle(&mut self, last: DateTime<Utc>) -> Result<()> {
        let age = Utc::now().signed_duration_since(last).to_std()?;
        match (&self.active, age > self.threshold) {
            (None, true) => self.active = Some(self.open(last).await?),
            (Some(id), false) => {
                self.close(id).await?;
                self.active = None;
            }
            _ => {}
        }
        Ok(())
    }

    async fn open(&self, last: DateTime<Utc>) -> Result<String> {
        let body = NewIncident {
            name: "No L2 head events – Possible Outage".into(),
            message: format!("No L2 head event for {} s", self.threshold.as_secs()),
            status: "INVESTIGATING".into(),
            components: vec![self.component_id.clone()],
            statuses: vec![ComponentStatus::major_outage(&self.component_id)],
            notify: true,
            started: Some(last.to_rfc3339()),
        };
        let id = self.client.create_incident(&body).await?;
        info!(%id, "Created incident");
        Ok(id)
    }

    async fn close(&self, id: &str) -> Result<()> {
        let body = ResolveIncident {
            message: "L2 head events have resumed.".into(),
            status: "RESOLVED".into(),
            components: vec![self.component_id.clone()],
            statuses: vec![ComponentStatus::operational(&self.component_id)],
            notify: true,
        };
        self.client.resolve_incident(id, &body).await?;
        info!(%id, "Resolved incident");
        Ok(())
    }
}
