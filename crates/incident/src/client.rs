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
    api_key: String,
    page_id: String,
}

impl Client {
    /// Create a new Instatus API client.
    pub fn new(api_key: String, page_id: String) -> Self {
        Self { http: HttpClient::new(), api_key, page_id }
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
        let url = format!("https://api.instatus.com/v1/{}/incidents", self.page_id);
        let resp = self.auth(self.http.post(&url)).json(body).send().await?.error_for_status()?;
        Ok(resp.json::<Resp>().await?.id)
    }

    /// Resolve an existing incident on Instatus.
    pub async fn resolve_incident(&self, id: &str, body: &ResolveIncident) -> Result<()> {
        let url = format!("https://api.instatus.com/v1/{}/incidents/{}", self.page_id, id);
        self.auth(self.http.post(&url)).json(body).send().await?.error_for_status()?;
        Ok(())
    }

    /// Return open incident ID for `component_id`, if any.
    pub async fn open_incident(&self, component_id: &str) -> Result<Option<String>> {
        let url =
            format!("https://api.instatus.com/v1/{}/incidents?status=INVESTIGATING", self.page_id);
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

    use crate::monitor::{ComponentStatus, IncidentState};

    use serde_json::json;

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
}
