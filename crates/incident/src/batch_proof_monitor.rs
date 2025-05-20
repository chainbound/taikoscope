use crate::{
    client::Client as IncidentClient,
    monitor::{ComponentStatus, IncidentState, NewIncident, ResolveIncident},
};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use clickhouse::ClickhouseClient;
use eyre::Result;
use std::{collections::HashMap, time::Duration};
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

/// Monitors batches that take too long to prove (> 3 hours after being posted).
/// Creates incidents for batches that have been posted but not proven within the time threshold.
#[derive(Debug)]
#[allow(dead_code)]
pub struct BatchProofTimeoutMonitor {
    clickhouse: ClickhouseClient,
    client: IncidentClient,
    component_id: String,
    proof_timeout: Duration,
    interval: Duration,
    active_incidents: HashMap<u64, String>, // Map of batch_id -> incident_id
    healthy_needed: u8,
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
            clickhouse,
            client,
            component_id,
            proof_timeout,
            interval,
            active_incidents: HashMap::new(),
            healthy_needed,
        }
    }

    /// Spawns the batch proof timeout monitor on the Tokio runtime.
    pub fn spawn(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            if let Err(e) = self.run().await {
                error!(%e, "batch proof timeout monitor exited unexpectedly");
            }
        })
    }

    async fn run(mut self) -> Result<()> {
        // Check for existing incidents
        match self.client.open_incident(&self.component_id).await? {
            Some(id) => {
                info!(incident_id = %id, component_id = %self.component_id,
                    "Found open batch proof timeout incident at startup, monitoring for resolution");
                // We can't determine which batch this belongs to, so we'll just monitor it
                // and close it if all outstanding batches get proven
                self.active_incidents.insert(0, id);
            }
            None => {
                info!(component_id = %self.component_id, "No open batch proof timeout incidents at startup")
            }
        }

        let mut interval = tokio::time::interval(self.interval);
        loop {
            interval.tick().await;
            if let Err(e) = self.check_unproven_batches().await {
                error!(%e, "error checking for unproven batches");
            }
        }
    }

    /// Check for batches that have not been proven within the timeout period
    async fn check_unproven_batches(&mut self) -> Result<()> {
        let cutoff_time = Utc::now() - ChronoDuration::from_std(self.proof_timeout)?;
        let unproven_batches = self.clickhouse.get_unproved_batches_older_than(cutoff_time).await?;

        debug!(
            "Found {} unproven batches older than {:?}",
            unproven_batches.len(),
            self.proof_timeout
        );

        for (_l1_block_number, batch_id, posted_at) in &unproven_batches {
            let age_hours = Utc::now().signed_duration_since(*posted_at).num_hours();
            if !self.active_incidents.contains_key(batch_id) {
                debug!(
                    batch_id = batch_id,
                    posted_at = %posted_at,
                    age_hours = age_hours,
                    "Found unproven batch exceeding timeout"
                );
                let incident_id =
                    self.open_incident(*batch_id, *posted_at, age_hours as u64).await?;
                self.active_incidents.insert(*batch_id, incident_id);
            }
        }

        // Now check if any active incidents should be resolved
        let mut resolved_batch_ids = Vec::new();
        for (batch_id, incident_id) in &self.active_incidents {
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
                self.resolve_incident(incident_id).await?;
                resolved_batch_ids.push(*batch_id);
            }
        }
        for batch_id in resolved_batch_ids {
            self.active_incidents.remove(&batch_id);
        }
        if self.active_incidents.len() == 1 && self.active_incidents.contains_key(&0) {
            if let Some(incident_id) = self.active_incidents.get(&0) {
                self.resolve_incident(incident_id).await?;
                self.active_incidents.remove(&0);
            }
        }
        Ok(())
    }

    /// Check if a specific batch has been proven
    async fn is_batch_proven(&self, batch_id: u64) -> Result<bool> {
        let proved_batch_ids = self.get_proved_batch_ids().await?;
        Ok(proved_batch_ids.contains(&batch_id))
    }

    async fn open_incident(
        &self,
        batch_id: u64,
        posted_at: DateTime<Utc>,
        age_hours: u64,
    ) -> Result<String> {
        let started = (posted_at +
            ChronoDuration::hours(self.proof_timeout.as_secs() as i64 / 3600))
        .to_rfc3339();

        let body = NewIncident {
            name: format!("Batch #{} Not Proven - Timeout", batch_id),
            message: format!(
                "Batch #{} has been waiting for proof for {}h (threshold: {}h)",
                batch_id,
                age_hours,
                self.proof_timeout.as_secs() / 3600
            ),
            status: IncidentState::Investigating,
            components: vec![self.component_id.clone()],
            statuses: vec![ComponentStatus::major_outage(&self.component_id)],
            notify: true,
            started: Some(started),
        };

        let id = self.client.create_incident(&body).await?;

        info!(
            incident_id = %id,
            batch_id = batch_id,
            name = %body.name,
            message = %body.message,
            status = ?body.status,
            components = ?body.components,
            "Created batch proof timeout incident"
        );

        Ok(id)
    }

    async fn resolve_incident(&self, id: &str) -> Result<()> {
        let body = ResolveIncident {
            status: IncidentState::Resolved,
            components: vec![self.component_id.clone()],
            statuses: vec![ComponentStatus::operational(&self.component_id)],
            notify: true,
            started: Some(Utc::now().to_rfc3339()),
        };

        debug!(%id, "Closing batch proof timeout incident");

        match self.client.resolve_incident(id, &body).await {
            Ok(_) => {
                info!(%id, "Successfully resolved batch proof timeout incident");
                Ok(())
            }
            Err(e) => {
                error!(%id, error = %e, "Failed to resolve batch proof timeout incident");
                Err(e)
            }
        }
    }

    /// Get all batch IDs that have been proven
    async fn get_proved_batch_ids(&self) -> Result<Vec<u64>> {
        self.clickhouse.get_proved_batch_ids().await
    }
}
