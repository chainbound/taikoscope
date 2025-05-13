use eyre::Result;
use reqwest::Client as HttpClient;
use serde::Deserialize;

use crate::monitor::{NewIncident, ResolveIncident};

#[derive(Deserialize)]
struct IncidentComponent {
    id: String,
    status: String,
    name: String,
}

#[derive(Deserialize)]
struct IncidentSummary {
    id: String,
    components: Vec<IncidentComponent>,
}

/// Client for interacting with the Instatus API.
#[derive(Debug, Clone)]
pub struct Client {
    http: HttpClient,
    base_url: String,
    api_key: String,
    page_id: String,
}

impl Client {
    /// Create a new Instatus API client.
    pub fn new(api_key: String, page_id: String) -> Self {
        Self {
            http: HttpClient::new(),
            base_url: "https://api.instatus.com".to_string(),
            api_key,
            page_id,
        }
    }

    /// Create a client targeting a custom base URL (e.g. for tests).
    #[cfg(test)]
    pub fn with_base_url(api_key: String, page_id: String, base_url: String) -> Self {
        Self { http: HttpClient::new(), api_key, page_id, base_url }
    }

    /// Authenticate the request.
    fn auth(&self, rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        rb.bearer_auth(&self.api_key)
    }

    /// Create a new incident on Instatus.
    pub async fn create_incident(&self, body: &NewIncident) -> Result<String> {
        #[derive(Deserialize)]
        struct Resp {
            id: String,
        }
        let url = format!("{}/v1/{}/incidents", self.base_url, self.page_id);
        let resp = self.auth(self.http.post(&url)).json(body).send().await?.error_for_status()?;
        Ok(resp.json::<Resp>().await?.id)
    }

    /// Resolve an existing incident on Instatus.
    pub async fn resolve_incident(&self, id: &str, body: &ResolveIncident) -> Result<()> {
        let url = format!("{}/v1/{}/incidents/{}", self.base_url, self.page_id, id);
        self.auth(self.http.post(&url)).json(body).send().await?.error_for_status()?;
        Ok(())
    }

    /// Return open incident ID for `component_id`, if any.
    pub async fn open_incident(&self, component_id: &str) -> Result<Option<String>> {
        let url = format!("{}/v1/{}/incidents?status=INVESTIGATING", self.base_url, self.page_id);
        let list = self
            .auth(self.http.get(&url))
            .send()
            .await?
            .error_for_status()?
            .json::<Vec<IncidentSummary>>()
            .await?;

        // find the first incident touching our component
        if let Some((incident_id, comp)) = list.into_iter().find_map(|inc| {
            inc.components.into_iter().find(|c| c.id == component_id).map(|comp| (inc.id, comp))
        }) {
            tracing::info!(
                incident_id = %incident_id,
                component_name = %comp.name,
                component_status = %comp.status,
                "Found open incident for component"
            );
            Ok(Some(incident_id))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::monitor::{ComponentStatus, IncidentState, NewIncident, ResolveIncident};

    use mockito::{Matcher, Server};
    use serde_json::json;
    use tokio;

    #[test]
    fn test_new_incident_serialization() {
        let payload = NewIncident {
            name: "No L2 head events - Possible Outage".to_string(),
            message: "No L2 head event for 30s".to_string(),
            status: IncidentState::Investigating,
            components: vec!["comp1".to_string()],
            statuses: vec![ComponentStatus {
                id: "comp1".to_string(),
                status: "MAJOROUTAGE".to_string(),
            }],
            notify: true,
            started: Some("2025-05-12T07:48:00Z".to_string()),
        };
        let expected = json!({
            "name": "No L2 head events - Possible Outage",
            "message": "No L2 head event for 30s",
            "status": "INVESTIGATING",
            "components": ["comp1"],
            "statuses": [{"id": "comp1", "status": "MAJOROUTAGE"}],
            "notify": true,
            "started": "2025-05-12T07:48:00Z"
        });
        let actual = serde_json::to_value(&payload).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_resolve_incident_serialization() {
        let payload = ResolveIncident {
            message: "L2 head events have resumed.".to_string(),
            status: IncidentState::Resolved,
            components: vec!["comp1".to_string()],
            statuses: vec![ComponentStatus {
                id: "comp1".to_string(),
                status: "OPERATIONAL".to_string(),
            }],
            notify: true,
        };
        let expected = json!({
            "message": "L2 head events have resumed.",
            "status": "RESOLVED",
            "components": ["comp1"],
            "statuses": [{"id": "comp1", "status": "OPERATIONAL"}],
            "notify": true
        });
        let actual = serde_json::to_value(&payload).unwrap();
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn create_incident_hits_correct_endpoint() {
        let mut server = Server::new_async().await;
        let expected = json!({
            "name": "Test incident",
            "message": "Testing",
            "status": "INVESTIGATING",
            "components": ["comp1"],
            "statuses": [{"id": "comp1", "status": "MAJOROUTAGE"}],
            "notify": true,
            "started": "2025-05-12T00:00:00Z"
        })
        .to_string();

        let mock = server
            .mock("POST", "/v1/page1/incidents")
            .match_header("authorization", "Bearer testkey")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Exact(expected))
            .with_status(200)
            .with_body(r#"{"id":"incident123"}"#)
            .create_async()
            .await;

        let client = Client::with_base_url("testkey".into(), "page1".into(), server.url());
        let payload = NewIncident {
            name: "Test incident".into(),
            message: "Testing".into(),
            status: IncidentState::Investigating,
            components: vec!["comp1".into()],
            statuses: vec![ComponentStatus::major_outage("comp1")],
            notify: true,
            started: Some("2025-05-12T00:00:00Z".into()),
        };
        let id = client.create_incident(&payload).await.unwrap();
        assert_eq!(id, "incident123");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn resolve_incident_hits_update_endpoint() {
        let mut server = Server::new_async().await;
        let expected = json!({
            "message": "Resolved",
            "status": "RESOLVED",
            "components": ["comp1"],
            "statuses": [{"id": "comp1", "status": "OPERATIONAL"}],
            "notify": true
        })
        .to_string();

        let mock = server
            .mock("POST", "/v1/page1/incidents/incident123")
            .match_header("authorization", "Bearer testkey")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Exact(expected))
            .with_status(200)
            .create_async()
            .await;

        let client = Client::with_base_url("testkey".into(), "page1".into(), server.url());
        let payload = ResolveIncident {
            message: "Resolved".into(),
            status: IncidentState::Resolved,
            components: vec!["comp1".into()],
            statuses: vec![ComponentStatus::operational("comp1")],
            notify: true,
        };
        client.resolve_incident("incident123", &payload).await.unwrap();
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn find_open_incident_for_component_filters_correctly() {
        let mut server = Server::new_async().await;
        let body = json!([
            { "id": "inc1", "components": [ { "id": "compX", "name": "X", "status": "OPERATIONAL" } ] },
            { "id": "inc2", "components": [ { "id": "comp1", "name": "Target", "status": "MAJOROUTAGE" } ] }
        ])
        .to_string();

        let mock = server
            .mock("GET", "/v1/page1/incidents")
            .match_query(Matcher::UrlEncoded("status".into(), "INVESTIGATING".into()))
            .with_status(200)
            .with_body(body)
            .create_async()
            .await;

        let client = Client::with_base_url("testkey".into(), "page1".into(), server.url());
        let id = client.open_incident("comp1").await.unwrap();
        assert_eq!(id, Some("inc2".into()));
        mock.assert_async().await;
    }
}
