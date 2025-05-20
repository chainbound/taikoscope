use crate::{
    client::Client as IncidentClient,
    monitor::{ComponentStatus, IncidentState, NewIncident, ResolveIncident},
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use clickhouse::ClickhouseClient;
use eyre::Result;
use std::fmt::Debug;
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

/// Monitor trait for different incident types
#[async_trait]
pub trait Monitor: Send + Sync {
    /// Incident key type (e.g. u64 for batch ID, () for global)
    type IncidentKey: Clone + Debug + Eq + std::hash::Hash;

    /// Creates a new incident
    async fn create_incident(&self, key: &Self::IncidentKey) -> Result<String>;

    /// Resolves an incident by its ID
    async fn resolve_incident(&self, incident_id: &str) -> Result<()>;

    /// Checks the health of the monitored component
    async fn check_health(&mut self) -> Result<()>;

    /// Initializes the monitor, checking for existing incidents
    async fn initialize(&mut self) -> Result<()>;

    /// Runs the monitor in a loop
    async fn run(self) -> Result<()>;

    /// Spawns the monitor on the Tokio runtime
    fn spawn(self) -> JoinHandle<()>
    where
        Self: Sized + 'static,
    {
        let monitor_name = std::any::type_name::<Self>();
        tokio::spawn(async move {
            if let Err(e) = self.run().await {
                error!(%e, monitor = monitor_name, "monitor exited unexpectedly");
            }
        })
    }

    /// Gets the polling interval for the monitor
    fn get_interval(&self) -> std::time::Duration;

    /// Gets the component ID for the monitor
    fn get_component_id(&self) -> &str;

    /// Gets the client for the monitor
    fn get_client(&self) -> &IncidentClient;

    /// Gets a reference to the `ClickHouse` client
    fn get_clickhouse(&self) -> &ClickhouseClient;
}

/// A base implementation for common monitor functionality
#[derive(Debug)]
pub struct BaseMonitor<K> {
    /// `ClickHouse` client for querying data
    pub clickhouse: ClickhouseClient,
    /// Incident client for creating and managing incidents
    pub client: IncidentClient,
    /// Component ID for the monitored component
    pub component_id: String,
    /// Monitoring interval
    pub interval: std::time::Duration,
    /// Map of active incidents
    pub active_incidents: std::collections::HashMap<K, String>,
    /// Number of consecutive healthy checks needed before resolving an incident
    pub healthy_needed: u8,
    /// Number of consecutive healthy checks seen
    pub healthy_seen: u8,
}

impl<K: Clone + Debug + Eq + std::hash::Hash> BaseMonitor<K> {
    /// Create a new base monitor
    pub fn new(
        clickhouse: ClickhouseClient,
        client: IncidentClient,
        component_id: String,
        interval: std::time::Duration,
        healthy_needed: u8,
    ) -> Self {
        Self {
            clickhouse,
            client,
            component_id,
            interval,
            active_incidents: std::collections::HashMap::new(),
            healthy_needed,
            healthy_seen: 0,
        }
    }

    /// Create a standard incident payload
    pub fn create_incident_payload(
        &self,
        name: String,
        message: String,
        started: DateTime<Utc>,
    ) -> NewIncident {
        NewIncident {
            name,
            message,
            status: IncidentState::Investigating,
            components: vec![self.component_id.clone()],
            statuses: vec![ComponentStatus::major_outage(&self.component_id)],
            notify: true,
            started: Some(started.to_rfc3339()),
        }
    }

    /// Create a standard resolve payload
    pub fn create_resolve_payload(&self) -> ResolveIncident {
        ResolveIncident {
            status: IncidentState::Resolved,
            components: vec![self.component_id.clone()],
            statuses: vec![ComponentStatus::operational(&self.component_id)],
            notify: true,
            started: Some(Utc::now().to_rfc3339()),
        }
    }

    /// Helper to create an incident
    pub async fn create_incident_with_payload(&self, payload: &NewIncident) -> Result<String> {
        let id = self.client.create_incident(payload).await?;

        info!(
            incident_id = %id,
            name = %payload.name,
            message = %payload.message,
            status = ?payload.status,
            components = ?payload.components,
            "Created incident"
        );

        Ok(id)
    }

    /// Helper to resolve an incident
    pub async fn resolve_incident_with_payload(
        &self,
        id: &str,
        payload: &ResolveIncident,
    ) -> Result<()> {
        debug!(%id, "Closing incident");

        match self.client.resolve_incident(id, payload).await {
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

    /// Helper method to check for existing open incidents for this component
    pub async fn check_existing_incidents(&mut self, default_key: K) -> Result<()> {
        match self.client.open_incident(&self.component_id).await? {
            Some(id) => {
                info!(
                    incident_id = %id,
                    component_id = %self.component_id,
                    "Found open incident at startup, monitoring for resolution"
                );
                self.active_incidents.insert(default_key, id);
            }
            None => {
                info!(
                    component_id = %self.component_id,
                    "No open incidents found at startup"
                );
            }
        }

        Ok(())
    }
}

impl<K: Clone + Debug + Eq + std::hash::Hash> BaseMonitor<K> {
    /// Helper method to mark an incident as healthy and potentially resolve it
    pub async fn mark_healthy(&mut self, key: &K) -> Result<bool> {
        if let Some(incident_id) = self.active_incidents.get(key) {
            self.healthy_seen += 1;

            if self.healthy_seen >= self.healthy_needed {
                let payload = self.create_resolve_payload();
                self.resolve_incident_with_payload(incident_id, &payload).await?;
                self.active_incidents.remove(key);
                self.healthy_seen = 0;
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Mark the monitor as unhealthy, resetting the healthy counter.
    pub const fn mark_unhealthy(&mut self) {
        self.healthy_seen = 0;
    }
}
