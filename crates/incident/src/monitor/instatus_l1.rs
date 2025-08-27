use crate::{
    base_monitor::{BaseMonitor, Monitor},
    client::Client as IncidentClient,
};
use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use clickhouse::ClickhouseReader;
use eyre::Result;
use std::time::Duration;
use tracing::{debug, error};

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
        if let Some(_id) = self.base.active_incidents.values().next() &&
            let (Ok(Some(batch_ts)), Ok(Some(l2_ts))) = (
                self.base.clickhouse.get_last_batch_time().await,
                self.base.clickhouse.get_last_l2_head_time().await,
            ) &&
            let Err(e) = self.handle(batch_ts, l2_ts).await
        {
            error!(%e, "Failed initial health check for existing batch incident");
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
