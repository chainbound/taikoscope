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

/// Monitors batches that take too long to prove (> 3 hours after being posted).
/// Creates incidents for batches that have been posted but not proven within the time threshold.
#[derive(Debug)]
pub struct BatchProofTimeoutMonitor {
    /// Base monitor implementation
    pub(crate) base: BaseMonitor<(u64, u64)>,
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
    pub(crate) fn filter_new_batches(
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
    pub(crate) fn catch_all_only(&self) -> bool {
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
