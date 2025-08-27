use crate::{
    client::Client as IncidentClient,
    monitor::{ComponentStatus, IncidentState, NewIncident, ResolveIncident},
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use clickhouse::ClickhouseReader;
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
    fn get_clickhouse(&self) -> &ClickhouseReader;
}

/// A base implementation for common monitor functionality
#[derive(Debug)]
pub struct BaseMonitor<K> {
    /// `ClickHouse` client for querying data
    pub clickhouse: ClickhouseReader,
    /// Incident client for creating and managing incidents
    pub client: IncidentClient,
    /// Component ID for the monitored component
    pub component_id: String,
    /// Monitoring interval
    pub interval: std::time::Duration,
    /// Whether reporting to Instatus is enabled.
    /// When disabled, monitors will still evaluate health and log warnings,
    /// but will not call the Instatus API. Useful for dry-run mode.
    pub reporting_enabled: bool,
    /// Map of active incidents
    pub active_incidents: std::collections::HashMap<K, String>,
}

impl<K: Clone + Debug + Eq + std::hash::Hash> BaseMonitor<K> {
    /// Create a new base monitor
    pub fn new(
        clickhouse: ClickhouseReader,
        client: IncidentClient,
        component_id: String,
        interval: std::time::Duration,
    ) -> Self {
        Self {
            clickhouse,
            client,
            component_id: component_id.clone(),
            interval,
            // If no component id is configured, treat reporting as disabled (dry-run)
            reporting_enabled: !component_id.is_empty(),
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
        crate::helpers::build_incident_payload(&self.component_id, name, message, started)
    }

    /// Create a standard resolve payload
    pub fn create_resolve_payload(&self) -> ResolveIncident {
        crate::helpers::build_resolve_payload(&self.component_id)
    }

    /// Helper to create an incident
    pub async fn create_incident_with_payload(&self, payload: &NewIncident) -> Result<String> {
        crate::helpers::create_with_retry(&self.client, self.reporting_enabled, payload).await
    }

    /// Helper to resolve an incident
    pub async fn resolve_incident_with_payload(
        &self,
        id: &str,
        payload: &ResolveIncident,
    ) -> Result<()> {
        crate::helpers::resolve_with_retry(&self.client, self.reporting_enabled, id, payload).await
    }

    /// Helper method to check for existing open incidents for this component
    pub async fn check_existing_incidents(&mut self, default_key: K) -> Result<()> {
        if !self.reporting_enabled {
            tracing::info!(
                component_id = %self.component_id,
                "Instatus monitors disabled - skipping check for existing incidents (dry-run)"
            );
            return Ok(());
        }

        match crate::retry::retry_op(|| async {
            self.client.open_incident(&self.component_id).await
        })
        .await?
        {
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
        if let Some(incident_id) = self.active_incidents.get(key).cloned() {
            tracing::info!(
                incident_id = %incident_id,
                key = ?key,
                "Attempting to resolve incident"
            );

            // Validate that the incident exists on the current page before attempting resolution
            match self.client.incident_exists(&incident_id).await {
                Ok(true) => {
                    // Incident exists, proceed with resolution
                    let payload = self.create_resolve_payload();
                    self.resolve_incident_with_payload(&incident_id, &payload).await?;
                    self.active_incidents.remove(key);
                    tracing::info!(
                        incident_id = %incident_id,
                        key = ?key,
                        "Successfully resolved incident"
                    );
                    return Ok(true);
                }
                Ok(false) => {
                    // Incident doesn't exist on current page, remove from tracking
                    tracing::warn!(
                        incident_id = %incident_id,
                        key = ?key,
                        "Incident not found on current page, removing from tracking"
                    );
                    self.active_incidents.remove(key);
                    return Ok(false);
                }
                Err(e) => {
                    // Error checking incident existence, log but don't retry resolution
                    tracing::error!(
                        incident_id = %incident_id,
                        key = ?key,
                        error = %e,
                        "Failed to validate incident existence, skipping resolution"
                    );
                    return Err(e);
                }
            }
        }

        tracing::debug!(
            key = ?key,
            active_incidents_count = %self.active_incidents.len(),
            "No active incident found for key"
        );
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::Client as IncidentClient;
    use chrono::Utc;
    use clickhouse::ClickhouseReader as ClickhouseInternalClient;
    use mockito::{Matcher, Server, ServerGuard};
    use std::time::Duration;
    use url::Url;

    fn mock_clickhouse_client() -> (ClickhouseInternalClient, ServerGuard) {
        let server = mockito::Server::new();
        let url = Url::parse(&server.url()).unwrap();
        let client = ClickhouseInternalClient::new(
            url,
            "test_db".to_owned(),
            "user".to_owned(),
            "pass".to_owned(),
        )
        .unwrap();
        (client, server)
    }

    fn mock_incident_client() -> (IncidentClient, ServerGuard) {
        let server = mockito::Server::new();
        let url = Url::parse(&server.url()).unwrap();
        let client = IncidentClient::with_base_url("testkey".to_owned(), "page1".to_owned(), url);
        (client, server)
    }

    async fn mock_clickhouse_client_async() -> (ClickhouseInternalClient, ServerGuard) {
        let server = Server::new_async().await;
        let url = Url::parse(&server.url()).unwrap();
        let client = ClickhouseInternalClient::new(
            url,
            "test_db".to_owned(),
            "user".to_owned(),
            "pass".to_owned(),
        )
        .unwrap();
        (client, server)
    }

    async fn mock_incident_client_async() -> (IncidentClient, ServerGuard) {
        let server = Server::new_async().await;
        let url = Url::parse(&server.url()).unwrap();
        let client = IncidentClient::with_base_url("testkey".to_owned(), "page1".to_owned(), url);
        (client, server)
    }

    #[test]
    fn create_incident_payload_builds_expected() {
        let (ch_client, _ch_server) = mock_clickhouse_client();
        let (incident_client, _inc_server) = mock_incident_client();
        let monitor = BaseMonitor::<u64>::new(
            ch_client,
            incident_client,
            "comp1".to_owned(),
            Duration::from_secs(1),
        );
        let started = Utc::now();
        let payload = monitor.create_incident_payload("name".into(), "msg".into(), started);
        assert_eq!(payload.name, "name");
        assert_eq!(payload.message, "msg");
        assert_eq!(payload.status, IncidentState::Investigating);
        assert_eq!(payload.components, vec!["comp1".to_owned()]);
        assert_eq!(payload.statuses, vec![ComponentStatus::major_outage("comp1")]);
        assert!(payload.notify);
        let expected = started.to_rfc3339();
        assert_eq!(payload.started.as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn create_resolve_payload_builds_expected() {
        let (ch_client, _ch_server) = mock_clickhouse_client();
        let (incident_client, _inc_server) = mock_incident_client();
        let monitor = BaseMonitor::<u64>::new(
            ch_client,
            incident_client,
            "comp1".to_owned(),
            Duration::from_secs(1),
        );
        let payload = monitor.create_resolve_payload();
        assert_eq!(payload.status, IncidentState::Resolved);
        assert_eq!(payload.components, vec!["comp1".to_owned()]);
        assert_eq!(payload.statuses, vec![ComponentStatus::operational("comp1")]);
        assert!(payload.notify);
        assert!(payload.started.is_some());
    }

    #[tokio::test]
    async fn create_incident_with_payload_hits_endpoint() {
        let (ch_client, _ch_server) = mock_clickhouse_client_async().await;
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/page1/incidents")
            .match_header("authorization", "Bearer testkey")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_body(r#"{"id":"inc123"}"#)
            .create_async()
            .await;

        let incident_client = IncidentClient::with_base_url(
            "testkey".into(),
            "page1".into(),
            server.url().parse().unwrap(),
        );
        let monitor = BaseMonitor::<u64>::new(
            ch_client,
            incident_client,
            "comp1".to_owned(),
            Duration::from_secs(1),
        );
        let payload = monitor.create_incident_payload("n".into(), "m".into(), Utc::now());
        let id = monitor.create_incident_with_payload(&payload).await.unwrap();
        assert_eq!(id, "inc123");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn resolve_incident_with_payload_success() {
        let (ch_client, _ch_server) = mock_clickhouse_client_async().await;
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/v1/page1/incidents/inc123")
            .match_header("authorization", "Bearer testkey")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_body("{}")
            .create_async()
            .await;

        let incident_client = IncidentClient::with_base_url(
            "testkey".into(),
            "page1".into(),
            server.url().parse().unwrap(),
        );
        let monitor = BaseMonitor::<u64>::new(
            ch_client,
            incident_client,
            "comp1".to_owned(),
            Duration::from_secs(1),
        );
        let payload = monitor.create_resolve_payload();
        monitor.resolve_incident_with_payload("inc123", &payload).await.unwrap();
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn resolve_incident_with_payload_error() {
        let (ch_client, _ch_server) = mock_clickhouse_client_async().await;
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/v1/page1/incidents/inc123")
            .match_header("authorization", "Bearer testkey")
            .match_header("content-type", "application/json")
            .with_status(500)
            .with_body("err")
            .expect_at_least(1)
            .create_async()
            .await;

        let incident_client = IncidentClient::with_base_url(
            "testkey".into(),
            "page1".into(),
            server.url().parse().unwrap(),
        );
        let monitor = BaseMonitor::<u64>::new(
            ch_client,
            incident_client,
            "comp1".to_owned(),
            Duration::from_secs(1),
        );
        let payload = monitor.create_resolve_payload();
        let err = monitor.resolve_incident_with_payload("inc123", &payload).await.unwrap_err();
        assert!(err.to_string().contains("500"));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn check_existing_incidents_inserts_on_match() {
        let (ch_client, _ch_server) = mock_clickhouse_client_async().await;
        let mut server = Server::new_async().await;
        let body = serde_json::json!([
            {"id":"inc1","components":[{"id":"comp1","status":"MAJOROUTAGE","name":"C"}]}
        ])
        .to_string();
        let mock = server
            .mock("GET", "/v1/page1/incidents")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_body(body)
            .create_async()
            .await;

        let incident_client = IncidentClient::with_base_url(
            "testkey".into(),
            "page1".into(),
            server.url().parse().unwrap(),
        );
        let mut monitor = BaseMonitor::new(
            ch_client,
            incident_client,
            "comp1".to_owned(),
            Duration::from_secs(1),
        );
        monitor.check_existing_incidents(5u64).await.unwrap();
        assert_eq!(monitor.active_incidents.get(&5u64), Some(&"inc1".to_owned()));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn check_existing_incidents_none() {
        let (ch_client, _ch_server) = mock_clickhouse_client_async().await;
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/v1/page1/incidents")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_body("[]")
            .create_async()
            .await;

        let incident_client = IncidentClient::with_base_url(
            "testkey".into(),
            "page1".into(),
            server.url().parse().unwrap(),
        );
        let mut monitor = BaseMonitor::<u64>::new(
            ch_client,
            incident_client,
            "comp1".to_owned(),
            Duration::from_secs(1),
        );
        monitor.check_existing_incidents(42u64).await.unwrap();
        assert!(monitor.active_incidents.is_empty());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn mark_healthy_resolves_and_removes() {
        let (ch_client, _ch_server) = mock_clickhouse_client_async().await;
        let mut server = Server::new_async().await;

        // Mock for incident_exists check
        let exists_mock = server
            .mock("GET", "/v1/page1/incidents/inc123")
            .match_header("authorization", "Bearer testkey")
            .with_status(200)
            .with_body("{\"id\":\"inc123\"}")
            .create_async()
            .await;

        // Mock for resolve_incident
        let resolve_mock = server
            .mock("PUT", "/v1/page1/incidents/inc123")
            .match_header("authorization", "Bearer testkey")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_body("{}")
            .create_async()
            .await;

        let incident_client = IncidentClient::with_base_url(
            "testkey".into(),
            "page1".into(),
            server.url().parse().unwrap(),
        );
        let mut monitor = BaseMonitor::new(
            ch_client,
            incident_client,
            "comp1".to_owned(),
            Duration::from_secs(1),
        );
        monitor.active_incidents.insert(1u64, "inc123".to_owned());
        assert!(monitor.mark_healthy(&1u64).await.unwrap());
        assert!(monitor.active_incidents.is_empty());
        exists_mock.assert_async().await;
        resolve_mock.assert_async().await;
    }

    #[tokio::test]
    async fn mark_healthy_returns_false_when_absent() {
        let (ch_client, _ch_server) = mock_clickhouse_client_async().await;
        let (incident_client, _guard) = mock_incident_client_async().await;
        let mut monitor = BaseMonitor::<u64>::new(
            ch_client,
            incident_client,
            "comp1".to_owned(),
            Duration::from_secs(1),
        );
        assert!(!monitor.mark_healthy(&1u64).await.unwrap());
    }
}
