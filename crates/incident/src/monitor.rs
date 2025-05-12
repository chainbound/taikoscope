use crate::client::{Client as IncidentClient, ComponentStatus, NewIncident, ResolveIncident};
use chrono::{DateTime, Utc};
use clickhouse::ClickhouseClient;
use eyre::Result;
use std::time::Duration;
use tracing::{error, info};

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
