use crate::{client::Client as IncidentClient, helpers};
use chrono::Utc;
use network::public_rpc_monitor::check_syncing;
use reqwest::{Client, Url};
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

/// Spawn a background task monitoring the provided public RPC endpoint.
/// If an `IncidentClient` is provided, incidents will be created and resolved
/// when the endpoint is unhealthy or recovers.
pub fn spawn_public_rpc_monitor(
    url: Url,
    incident: Option<(IncidentClient, String)>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let client = Client::new();
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        let mut incident_id: Option<String> = None;
        loop {
            interval.tick().await;
            if let Some((ic, cid)) = &incident {
                check_once(&client, &url, Some((ic, cid)), &mut incident_id).await;
            } else {
                check_once(&client, &url, None, &mut incident_id).await;
            }
        }
    })
}

async fn check_once(
    client: &Client,
    url: &Url,
    incident: Option<(&IncidentClient, &String)>,
    incident_id: &mut Option<String>,
) {
    let first = check_syncing(client, url).await;
    let negative = match first {
        Ok(false) => {
            info!(url = url.as_str(), "public rpc healthy");
            if let Some((ic, cid)) = incident &&
                let Some(id) = incident_id.take()
            {
                resolve(ic, cid, &id).await;
            }
            false
        }
        Ok(true) => {
            warn!(url = url.as_str(), "public rpc syncing");
            true
        }
        Err(e) => {
            // Include error chain with debug formatting
            warn!(error = ?e, url = url.as_str(), "public rpc check failed");
            true
        }
    };

    if negative {
        tokio::time::sleep(Duration::from_secs(15)).await;
        match check_syncing(client, url).await {
            Ok(false) => {
                info!(url = url.as_str(), "public rpc recovered");
                if let Some((ic, cid)) = incident &&
                    let Some(id) = incident_id.take()
                {
                    resolve(ic, cid, &id).await;
                }
            }
            Ok(true) => {
                error!(url = url.as_str(), "public rpc still syncing");
                if let Some((ic, cid)) = incident {
                    open_if_needed(ic, cid, incident_id).await;
                }
            }
            Err(e) => {
                error!(error = ?e, url = url.as_str(), "public rpc check failed again");
                if let Some((ic, cid)) = incident {
                    open_if_needed(ic, cid, incident_id).await;
                }
            }
        }
    }
}

async fn open_if_needed(
    client: &IncidentClient,
    component_id: &str,
    incident_id: &mut Option<String>,
) {
    if incident_id.is_some() {
        return;
    }
    match client.open_incident(component_id).await {
        Ok(Some(id)) => {
            info!(incident_id = %id, "existing incident found, skipping creation");
            *incident_id = Some(id);
        }
        Ok(None) => {
            let body = helpers::build_incident_payload(
                component_id,
                "Public RPC Unavailable".to_owned(),
                "Public RPC endpoint is unreachable or syncing".to_owned(),
                Utc::now(),
            );
            match helpers::create_with_retry(client, true, &body).await {
                Ok(id) => {
                    info!(incident_id = %id, "created public rpc incident");
                    *incident_id = Some(id);
                }
                Err(e) => error!(error = %e, "failed to create incident"),
            }
        }
        Err(e) => error!(error = %e, "failed to query incidents"),
    }
}

async fn resolve(client: &IncidentClient, component_id: &str, id: &str) {
    let body = helpers::build_resolve_payload(component_id);
    if let Err(e) = helpers::resolve_with_retry(client, true, id, &body).await {
        error!(error = %e, incident_id = %id, "failed to resolve incident");
    }
}
