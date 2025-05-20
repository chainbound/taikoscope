use crate::{
    base_monitor::{BaseMonitor, Monitor},
    client::Client as IncidentClient,
};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use clickhouse::ClickhouseClient;
use eyre::Result;
use std::time::Duration;
use tracing::{debug, info};

/// Monitors batches that take too long to prove (> 3 hours after being posted).
/// Creates incidents for batches that have been posted but not proven within the time threshold.
#[derive(Debug)]
pub struct BatchProofTimeoutMonitor {
    /// Base monitor implementation
    base: BaseMonitor<u64>,
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
        healthy_needed: u8,
    ) -> Self {
        Self {
            base: BaseMonitor::new(clickhouse, client, component_id, interval, healthy_needed),
            proof_timeout,
        }
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
        let _started = (posted_at +
            chrono::Duration::hours(self.proof_timeout.as_secs() as i64 / 3600))
        .to_rfc3339();

        let body = self.base.create_incident_payload(
            format!("Batch #{} Not Proven - Timeout", batch_id),
            format!(
                "Batch #{} has been waiting for proof for {}h (threshold: {}h)",
                batch_id,
                age_hours,
                self.proof_timeout.as_secs() / 3600
            ),
            posted_at + ChronoDuration::hours(self.proof_timeout.as_secs() as i64 / 3600),
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
        for (_l1_block_number, batch_id, posted_at) in &unproven_batches {
            let age_hours = Utc::now().signed_duration_since(*posted_at).num_hours();
            if !self.base.active_incidents.contains_key(batch_id) {
                debug!(
                    batch_id = batch_id,
                    posted_at = %posted_at,
                    age_hours = age_hours,
                    "Found unproven batch exceeding timeout"
                );
                let incident_id =
                    self.open_incident(*batch_id, *posted_at, age_hours as u64).await?;
                self.base.active_incidents.insert(*batch_id, incident_id);
            }
        }

        // Check if any active incidents should be resolved
        let mut resolved_batch_ids = Vec::new();
        for (batch_id, incident_id) in &self.base.active_incidents {
            if *batch_id == 0 {
                continue;
            }
            let is_proven = self.is_batch_proven(*batch_id).await?;
            if is_proven {
                debug!(
                    batch_id = batch_id,
                    incident_id = %incident_id,
                    "Batch is now proven, resolving incident"
                );
                let payload = self.base.create_resolve_payload();
                self.base.resolve_incident_with_payload(incident_id, &payload).await?;
                resolved_batch_ids.push(*batch_id);
            }
        }

        for batch_id in resolved_batch_ids {
            self.base.active_incidents.remove(&batch_id);
        }

        // Special case for the catch-all incident (batch_id = 0)
        if self.base.active_incidents.len() == 1 && self.base.active_incidents.contains_key(&0) {
            if let Some(incident_id) = self.base.active_incidents.get(&0) {
                let payload = self.base.create_resolve_payload();
                self.base.resolve_incident_with_payload(incident_id, &payload).await?;
                self.base.active_incidents.remove(&0);
            }
        }
        Ok(())
    }
}

impl Monitor for BatchProofTimeoutMonitor {
    type IncidentKey = u64;

    fn create_incident(
        &self,
        key: &Self::IncidentKey,
        _data: &impl std::fmt::Debug,
    ) -> impl std::future::Future<Output = Result<String>> + Send {
        async move { self.open_incident(*key, Utc::now(), 0).await }
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
        async move { self.check_unproven_batches().await }
    }

    fn initialize(&mut self) -> impl std::future::Future<Output = Result<()>> + Send {
        async move {
            // Check for existing incidents
            self.base.check_existing_incidents(0).await
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
