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
}

impl<K: Clone + Debug + Eq + std::hash::Hash> BaseMonitor<K> {
    /// Create a new base monitor
    pub fn new(
        clickhouse: ClickhouseClient,
        client: IncidentClient,
        component_id: String,
        interval: std::time::Duration,
    ) -> Self {
        Self {
            clickhouse,
            client,
            component_id,
            interval,
            active_incidents: std::collections::HashMap::new(),
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
            let payload = self.create_resolve_payload();
            self.resolve_incident_with_payload(incident_id, &payload).await?;
            self.active_incidents.remove(key);
            return Ok(true);
        }

        Ok(false)
    }

    /// Mark the monitor as unhealthy, dropping the healthy counter behavior.
    pub const fn mark_unhealthy(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::{ComponentStatus, IncidentState, NewIncident, ResolveIncident};
    use async_trait::async_trait;
    use chrono::Utc;
    use clickhouse::ClickhouseClient as ActualClickhouseClient; // Renamed to avoid conflict
    use mockall::mock;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use uuid::Uuid; // Added for generating test incident IDs

    // Mock for ClickhouseClient
    mock! {
        pub ClickhouseClient { // Matches the name used in BaseMonitor
            fn new(url: &str) -> Self; // Assuming a new method, adjust if different
            // Add other methods used by BaseMonitor if any, for now, keeping it simple
        }
    }

    // Mock for IncidentClient
    mock! {
        pub Client { // Matches the name used in BaseMonitor (Client as IncidentClient)
            fn new(server_url: &str, api_key: &str) -> Self; // Assuming a new method

            async fn create_incident(&self, incident: &NewIncident) -> Result<String>;
            async fn resolve_incident(&self, incident_id: &str, resolution: &ResolveIncident) -> Result<()>;
            async fn open_incident(&self, component_id: &str) -> Result<Option<String>>;
            // Add other methods if BaseMonitor starts using them
        }
    }

    // Helper function to create a mock IncidentClient
    fn mock_incident_client() -> Client {
        MockClient::new()
    }

    // Helper function to create a mock ClickhouseClient
    fn mock_clickhouse_client() -> ActualClickhouseClient {
        // ActualClickhouseClient::new("dummy_url") // This might need adjustment based on actual ClickhouseClient constructor
        // For now, if BaseMonitor only needs the type, we might not need to fully construct it,
        // or use a simpler mock if direct construction is complex / has side effects.
        // Using a default mock for now.
        MockClickhouseClient::new("dummy_url").into() // into() might be needed if MockClickhouseClient doesn't directly return ActualClickhouseClient
    }


    #[test]
    fn test_base_monitor_new() {
        let mock_ch_client = mock_clickhouse_client();
        let mock_incident_client = mock_incident_client();
        let component_id = "test_component".to_string();
        let interval = std::time::Duration::from_secs(60);

        let monitor: BaseMonitor<u64> = BaseMonitor::new(
            mock_ch_client,
            mock_incident_client,
            component_id.clone(),
            interval,
        );

        assert_eq!(monitor.component_id, component_id);
        assert_eq!(monitor.interval, interval);
        assert!(monitor.active_incidents.is_empty());
        // Cannot directly compare clients as they are mocks.
        // We trust that they are stored if the other fields are correct.
    }

    #[test]
    fn test_create_incident_payload() {
        let mock_ch_client = mock_clickhouse_client();
        let mock_incident_client = mock_incident_client();
        let component_id = "payload_test_component".to_string();
        let interval = std::time::Duration::from_secs(60);
        let monitor: BaseMonitor<u64> = BaseMonitor::new(
            mock_ch_client,
            mock_incident_client,
            component_id.clone(),
            interval,
        );

        let name = "Test Incident".to_string();
        let message = "This is a test incident message.".to_string();
        let start_time = Utc::now();

        let payload = monitor.create_incident_payload(name.clone(), message.clone(), start_time);

        assert_eq!(payload.name, name);
        assert_eq!(payload.message, message);
        assert_eq!(payload.status, IncidentState::Investigating);
        assert_eq!(payload.components, vec![component_id.clone()]);
        assert_eq!(
            payload.statuses,
            vec![ComponentStatus::major_outage(&component_id)]
        );
        assert!(payload.notify);
        assert_eq!(payload.started, Some(start_time.to_rfc3339()));
    }

    #[test]
    fn test_create_resolve_payload() {
        let mock_ch_client = mock_clickhouse_client();
        let mock_incident_client = mock_incident_client();
        let component_id = "resolve_payload_test_component".to_string();
        let interval = std::time::Duration::from_secs(60);
        let monitor: BaseMonitor<u64> = BaseMonitor::new(
            mock_ch_client,
            mock_incident_client,
            component_id.clone(),
            interval,
        );

        let payload = monitor.create_resolve_payload();

        assert_eq!(payload.status, IncidentState::Resolved);
        assert_eq!(payload.components, vec![component_id.clone()]);
        assert_eq!(
            payload.statuses,
            vec![ComponentStatus::operational(&component_id)]
        );
        assert!(payload.notify);
        assert!(payload.started.is_some()); // Check that a start time is set
    }

    #[tokio::test]
    async fn test_create_incident_with_payload_success() {
        let mock_ch_client = mock_clickhouse_client();
        let mut mock_incident_client = mock_incident_client(); // Mutable to set expectations
        let component_id = "create_with_payload_test".to_string();
        let interval = std::time::Duration::from_secs(60);

        let expected_incident_id = "incident_123".to_string();
        let incident_name = "High CPU Usage".to_string();
        let incident_message = "CPU usage is above 90%".to_string();
        let start_time = Utc::now();

        let payload = NewIncident {
            name: incident_name.clone(),
            message: incident_message.clone(),
            status: IncidentState::Investigating,
            components: vec![component_id.clone()],
            statuses: vec![ComponentStatus::major_outage(&component_id)],
            notify: true,
            started: Some(start_time.to_rfc3339()),
        };

        // Expect create_incident to be called once with any NewIncident payload,
        // and return the expected ID.
        // We'll check the payload details if possible, or rely on the function's internal logic
        // which we assume is correct if it passes the payload along.
        let expected_id_clone = expected_incident_id.clone();
        mock_incident_client
            .expect_create_incident()
            .withf(move |p: &NewIncident| {
                p.name == incident_name && p.message == incident_message
                // It's a bit tricky to compare the whole payload due to potential subtle differences
                // like component_id clone, so we focus on key fields or trust BaseMonitor to construct it right.
            })
            .times(1)
            .returning(move |_| Ok(expected_id_clone.clone()));

        let monitor: BaseMonitor<u64> = BaseMonitor::new(
            mock_ch_client,
            mock_incident_client,
            component_id.clone(),
            interval,
        );

        let result = monitor.create_incident_with_payload(&payload).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_incident_id);
    }

    #[tokio::test]
    async fn test_create_incident_with_payload_error() {
        let mock_ch_client = mock_clickhouse_client();
        let mut mock_incident_client = mock_incident_client();
        let component_id = "create_error_test".to_string();
        let interval = std::time::Duration::from_secs(60);

        let incident_name = "Error Incident".to_string();
        let incident_message = "This should fail".to_string();
        let start_time = Utc::now();

        let payload = NewIncident {
            name: incident_name.clone(),
            message: incident_message.clone(),
            status: IncidentState::Investigating,
            components: vec![component_id.clone()],
            statuses: vec![ComponentStatus::major_outage(&component_id)],
            notify: true,
            started: Some(start_time.to_rfc3339()),
        };

        mock_incident_client
            .expect_create_incident()
            .times(1)
            .returning(|_| Err(eyre::eyre!("Failed to create incident")));

        let monitor: BaseMonitor<u64> = BaseMonitor::new(
            mock_ch_client,
            mock_incident_client,
            component_id.clone(),
            interval,
        );

        let result = monitor.create_incident_with_payload(&payload).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_resolve_incident_with_payload_success() {
        let mock_ch_client = mock_clickhouse_client();
        let mut mock_incident_client = mock_incident_client();
        let component_id = "resolve_with_payload_test".to_string();
        let interval = std::time::Duration::from_secs(60);
        let incident_id = "incident_to_resolve_123".to_string();

        let payload = ResolveIncident {
            status: IncidentState::Resolved,
            components: vec![component_id.clone()],
            statuses: vec![ComponentStatus::operational(&component_id)],
            notify: true,
            started: Some(Utc::now().to_rfc3339()), // Actual time doesn't matter much for mock
        };

        let id_clone = incident_id.clone();
        mock_incident_client
            .expect_resolve_incident()
            .withf(move |id: &str, p: &ResolveIncident| {
                id == id_clone && p.status == IncidentState::Resolved
                // Similar to create_incident, detailed payload check can be tricky.
                // Focusing on key fields.
            })
            .times(1)
            .returning(|_, _| Ok(()));

        let monitor: BaseMonitor<u64> = BaseMonitor::new(
            mock_ch_client,
            mock_incident_client,
            component_id.clone(),
            interval,
        );

        let result = monitor
            .resolve_incident_with_payload(&incident_id, &payload)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_resolve_incident_with_payload_error() {
        let mock_ch_client = mock_clickhouse_client();
        let mut mock_incident_client = mock_incident_client();
        let component_id = "resolve_error_test".to_string();
        let interval = std::time::Duration::from_secs(60);
        let incident_id = "incident_to_fail_resolve_123".to_string();

        let payload = ResolveIncident {
            status: IncidentState::Resolved,
            components: vec![component_id.clone()],
            statuses: vec![ComponentStatus::operational(&component_id)],
            notify: true,
            started: Some(Utc::now().to_rfc3339()),
        };

        mock_incident_client
            .expect_resolve_incident()
            .times(1)
            .returning(|_, _| Err(eyre::eyre!("Failed to resolve incident")));

        let monitor: BaseMonitor<u64> = BaseMonitor::new(
            mock_ch_client,
            mock_incident_client,
            component_id.clone(),
            interval,
        );

        let result = monitor
            .resolve_incident_with_payload(&incident_id, &payload)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_check_existing_incidents_found() {
        let mock_ch_client = mock_clickhouse_client();
        let mut mock_incident_client = mock_incident_client();
        let component_id = "check_existing_found_test".to_string();
        let interval = std::time::Duration::from_secs(60);
        let existing_incident_id = "existing_incident_456".to_string();
        let default_key: u64 = 1;

        let comp_id_clone = component_id.clone();
        let existing_id_clone = existing_incident_id.clone();
        mock_incident_client
            .expect_open_incident()
            .withf(move |id: &str| id == comp_id_clone)
            .times(1)
            .returning(move |_| Ok(Some(existing_id_clone.clone())));

        let mut monitor: BaseMonitor<u64> = BaseMonitor::new(
            mock_ch_client,
            mock_incident_client,
            component_id.clone(),
            interval,
        );

        let result = monitor.check_existing_incidents(default_key).await;

        assert!(result.is_ok());
        assert_eq!(monitor.active_incidents.len(), 1);
        assert_eq!(
            monitor.active_incidents.get(&default_key),
            Some(&existing_incident_id)
        );
    }

    #[tokio::test]
    async fn test_check_existing_incidents_not_found() {
        let mock_ch_client = mock_clickhouse_client();
        let mut mock_incident_client = mock_incident_client();
        let component_id = "check_existing_not_found_test".to_string();
        let interval = std::time::Duration::from_secs(60);
        let default_key: u64 = 1;

        let comp_id_clone = component_id.clone();
        mock_incident_client
            .expect_open_incident()
            .withf(move |id: &str| id == comp_id_clone)
            .times(1)
            .returning(|_| Ok(None));

        let mut monitor: BaseMonitor<u64> = BaseMonitor::new(
            mock_ch_client,
            mock_incident_client,
            component_id.clone(),
            interval,
        );

        let result = monitor.check_existing_incidents(default_key).await;

        assert!(result.is_ok());
        assert!(monitor.active_incidents.is_empty());
    }

    #[tokio::test]
    async fn test_mark_healthy_incident_exists() {
        let mock_ch_client = mock_clickhouse_client();
        let mut mock_incident_client = mock_incident_client();
        let component_id = "mark_healthy_exists_test".to_string();
        let interval = std::time::Duration::from_secs(60);
        let incident_key: u64 = 1;
        let incident_id = "active_incident_789".to_string();

        // Expect resolve_incident to be called
        let id_clone = incident_id.clone();
        mock_incident_client
            .expect_resolve_incident()
            .withf(move |id: &str, p: &ResolveIncident| {
                id == id_clone && p.status == IncidentState::Resolved // Basic check
            })
            .times(1)
            .returning(|_, _| Ok(()));

        let mut monitor: BaseMonitor<u64> = BaseMonitor::new(
            mock_ch_client,
            mock_incident_client,
            component_id.clone(),
            interval,
        );

        // Manually add an active incident
        monitor
            .active_incidents
            .insert(incident_key, incident_id.clone());

        let result = monitor.mark_healthy(&incident_key).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true); // Incident was resolved
        assert!(monitor.active_incidents.is_empty()); // Incident removed
    }

    #[tokio::test]
    async fn test_mark_healthy_no_incident() {
        let mock_ch_client = mock_clickhouse_client();
        let mut mock_incident_client = mock_incident_client(); // mut is not strictly needed here as no calls are expected
        let component_id = "mark_healthy_none_test".to_string();
        let interval = std::time::Duration::from_secs(60);
        let incident_key: u64 = 1; // An arbitrary key

        // No calls to resolve_incident are expected
        mock_incident_client.expect_resolve_incident().never();

        let mut monitor: BaseMonitor<u64> = BaseMonitor::new(
            mock_ch_client,
            mock_incident_client,
            component_id.clone(),
            interval,
        );

        // active_incidents is empty by default
        let result = monitor.mark_healthy(&incident_key).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false); // No incident was resolved
        assert!(monitor.active_incidents.is_empty()); // Still empty
    }

    #[test]
    fn test_mark_unhealthy() {
        let mock_ch_client = mock_clickhouse_client();
        let mock_incident_client = mock_incident_client();
        let component_id = "mark_unhealthy_test".to_string();
        let interval = std::time::Duration::from_secs(60);

        let mut monitor: BaseMonitor<u64> = BaseMonitor::new(
            mock_ch_client,
            mock_incident_client,
            component_id.clone(),
            interval,
        );

        // Call the function - the main test is that it doesn't panic
        // and completes. Since it's a `const fn` with an empty body currently,
        // there's no state change to assert.
        monitor.mark_unhealthy();

        // Optionally, assert that active_incidents is still empty,
        // though mark_unhealthy is not expected to change it.
        assert!(monitor.active_incidents.is_empty());
    }
}
