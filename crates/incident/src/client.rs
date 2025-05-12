use reqwest::Client as HttpClient;
use serde::Serialize;

/// Client for interacting with the Instatus API.
#[derive(Debug, Clone)]
pub struct Client {
    http: HttpClient,
    api_key: String,
    page_id: String,
}

impl Client {
    /// Create a new Instatus API client.
    pub fn new(api_key: String, page_id: String, _component_id: String) -> Self {
        Self { http: HttpClient::new(), api_key, page_id }
    }

    /// Create a new incident on Instatus.
    pub async fn create_incident(&self, payload: NewIncident) -> Result<String, reqwest::Error> {
        let url = format!("https://api.instatus.com/v1/{}/incidents", self.page_id);
        let resp = self.http.post(&url).bearer_auth(&self.api_key).json(&payload).send().await?;
        let json: serde_json::Value = resp.json().await?;
        Ok(json["id"].as_str().unwrap_or("").to_string())
    }

    /// Resolve an existing incident on Instatus.
    pub async fn resolve_incident(
        &self,
        incident_id: &str,
        payload: ResolveIncident,
    ) -> Result<(), reqwest::Error> {
        let url = format!(
            "https://api.instatus.com/v1/{}/incidents/{}/incident-updates",
            self.page_id, incident_id
        );
        self.http.post(&url).bearer_auth(&self.api_key).json(&payload).send().await?;
        Ok(())
    }
}

/// Payload for creating a new incident.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct NewIncident {
    /// Incident name
    pub name: String,
    /// Incident message/description
    pub message: String,
    /// Incident status (e.g. INVESTIGATING)
    pub status: String,
    /// Affected component IDs
    pub components: Vec<String>,
    /// Component statuses
    pub statuses: Vec<ComponentStatus>,
    /// Whether to notify subscribers
    pub notify: bool,
    /// Optional start timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started: Option<String>,
}

/// Payload for resolving an incident.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ResolveIncident {
    /// Update message
    pub message: String,
    /// Status (should be RESOLVED)
    pub status: String,
    /// Affected component IDs
    pub components: Vec<String>,
    /// Component statuses
    pub statuses: Vec<ComponentStatus>,
    /// Whether to notify subscribers
    pub notify: bool,
}

/// Status for a single component.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ComponentStatus {
    /// Component ID
    pub id: String,
    /// Status (e.g. MAJOROUTAGE, OPERATIONAL)
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_new_incident_serialization() {
        let payload = NewIncident {
            name: "No L2 head events – Possible Outage".to_string(),
            message: "No L2 head event for 30s".to_string(),
            status: "INVESTIGATING".to_string(),
            components: vec!["comp1".to_string()],
            statuses: vec![ComponentStatus {
                id: "comp1".to_string(),
                status: "MAJOROUTAGE".to_string(),
            }],
            notify: true,
            started: Some("2025-05-12T07:48:00Z".to_string()),
        };
        let expected = json!({
            "name": "No L2 head events – Possible Outage",
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
            status: "RESOLVED".to_string(),
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
