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
        // Efficient query to find batches posted but not proven
        // Join batches with proved_batches, find ones with insertion_time > 3 hours ago
        // that don't have a corresponding entry in proved_batches
        // Find batches that were proposed more than the timeout period ago
        let timeout_hours = self.proof_timeout.as_secs() / 3600; // convert seconds to hours
        let cutoff_time = Utc::now() - ChronoDuration::hours(timeout_hours as i64);

        // Execute queries to get unproven batches
        // We'll do this in two steps: 1) get all batches from before our cutoff time, 2) check
        // which ones haven't been proven yet
        let unproven_batches = self.find_unproven_batches(cutoff_time).await?;

        debug!(
            "Found {} unproven batches older than {} hours",
            unproven_batches.len(),
            timeout_hours
        );

        // Process each unproven batch
        for (batch_id, posted_at, age_ms) in &unproven_batches {
            let age_hours = *age_ms / (1000 * 60 * 60); // Convert ms to hours

            // Check if we already have an incident for this batch
            if !self.active_incidents.contains_key(batch_id) {
                debug!(
                    batch_id = batch_id,
                    posted_at = %posted_at,
                    age_hours = age_hours,
                    "Found unproven batch exceeding timeout"
                );

                let incident_id = self.open_incident(*batch_id, *posted_at, age_hours).await?;
                self.active_incidents.insert(*batch_id, incident_id);
            }
        }

        // Now check if any active incidents should be resolved
        // (batches were proven after we opened an incident)
        let mut resolved_batch_ids = Vec::new();

        for (batch_id, incident_id) in &self.active_incidents {
            // Skip the "unknown batch" incident (id 0) for now
            if *batch_id == 0 {
                continue;
            }

            // Check if the batch is now proven
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

        // Remove resolved incidents from our tracking
        for batch_id in resolved_batch_ids {
            self.active_incidents.remove(&batch_id);
        }

        // If we have no more active batch-specific incidents but still have
        // the unknown batch incident (id 0), resolve it too
        if self.active_incidents.len() == 1 && self.active_incidents.contains_key(&0) {
            if let Some(incident_id) = self.active_incidents.get(&0) {
                self.resolve_incident(incident_id).await?;
                self.active_incidents.remove(&0);
            }
        }

        Ok(())
    }

    /// Find batches that have been posted but not yet proven, and are older than the cutoff time
    async fn find_unproven_batches(
        &self,
        cutoff_time: DateTime<Utc>,
    ) -> Result<Vec<(u64, DateTime<Utc>, u64)>> {
        // Get all batches posted before the cutoff time
        let mut unproven_batches = Vec::new();

        // Get all batches from the database
        let all_batches = self.get_all_batches().await?;

        // Get all batch_ids that have been proven
        let proved_batch_ids = self.get_proved_batch_ids().await?;

        // Find batches that haven't been proven yet and are older than the cutoff
        for (batch_id, posted_at) in all_batches {
            if posted_at < cutoff_time && !proved_batch_ids.contains(&batch_id) {
                let age_ms = Utc::now().signed_duration_since(posted_at).num_milliseconds() as u64;
                unproven_batches.push((batch_id, posted_at, age_ms));
            }
        }

        Ok(unproven_batches)
    }

    /// Get all batches and their posted timestamps
    async fn get_all_batches(&self) -> Result<Vec<(u64, DateTime<Utc>)>> {
        // Create a simple query that gets all batch_ids and their insertion timestamps
        // TODO: Implement a proper query to ClickHouse
        // This is a temporary implementation for demonstration/testing

        // For example:
        // 1. Create a query to get all batches with their timestamps
        // 2. Execute the query using the ClickhouseClient
        // 3. Parse the results

        // Dummy implementation - replace with actual ClickHouse query
        let batches = vec![
            (1, Utc::now() - ChronoDuration::hours(4)), // 4 hours old batch
            (2, Utc::now() - ChronoDuration::hours(5)), // 5 hours old batch
        ];

        Ok(batches)
    }

    /// Get all batch IDs that have been proven
    async fn get_proved_batch_ids(&self) -> Result<Vec<u64>> {
        // Create a simple query that gets all proved batch_ids
        // TODO: Implement a proper query to ClickHouse
        // This is a temporary implementation for demonstration/testing

        // For example:
        // 1. Create a query to get all batch_ids from the proved_batches table
        // 2. Execute the query using the ClickhouseClient
        // 3. Parse the results

        // Dummy implementation - replace with actual ClickHouse query
        let proved_batches = vec![1]; // Batch 1 is proven

        Ok(proved_batches)
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
}
