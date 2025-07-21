use crate::{
    base_monitor::{BaseMonitor, Monitor},
    client::Client as IncidentClient,
};
use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use clickhouse::ClickhouseReader;
use eyre::Result;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Monitors `ClickHouse` L2 head events and manages Instatus incidents.
/// Polls `ClickHouse` every `interval` seconds; if no L2 head event for `threshold` seconds, it
/// creates an incident; resolves when events resume.
#[derive(Debug)]
pub struct InstatusMonitor {
    pub(crate) base: BaseMonitor<()>,
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
    pub(crate) async fn handle(&mut self, last: DateTime<Utc>) -> Result<()> {
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
