use crate::{
    base_monitor::{BaseMonitor, Monitor},
    client::Client as IncidentClient,
};
use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use clickhouse::ClickhouseReader;
use eyre::Result;
use std::time::Duration;
use tracing::{debug, error, info};

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
            none_left &&
            let Some(incident_id) = self.base.active_incidents.get(&catch_all_key)
        {
            info!(
                incident_id = %incident_id,
                "Resolving general batch verification timeout incident as all specific batches are clear or verified."
            );
            let payload = self.base.create_resolve_payload();
            self.base.resolve_incident_with_payload(incident_id, &payload).await?;
            self.base.active_incidents.remove(&catch_all_key);
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
