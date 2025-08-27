//! Shared helpers for incident payloads and operations with retry.
use chrono::{DateTime, Utc};
use eyre::Result;
use tracing::{debug, error, info, warn};

use crate::{
    client::Client as IncidentClient,
    monitor::{ComponentStatus, IncidentState, NewIncident, ResolveIncident},
    retry::retry_op,
};

/// Build a standard incident creation payload for a given component.
pub fn build_incident_payload(
    component_id: &str,
    name: String,
    message: String,
    started: DateTime<Utc>,
) -> NewIncident {
    NewIncident {
        name,
        message,
        status: IncidentState::Investigating,
        components: vec![component_id.to_owned()],
        statuses: vec![ComponentStatus::major_outage(component_id)],
        notify: true,
        started: Some(started.to_rfc3339()),
    }
}

/// Build a standard incident resolve payload for a given component.
pub fn build_resolve_payload(component_id: &str) -> ResolveIncident {
    ResolveIncident {
        status: IncidentState::Resolved,
        components: vec![component_id.to_owned()],
        statuses: vec![ComponentStatus::operational(component_id)],
        notify: true,
        started: Some(Utc::now().to_rfc3339()),
    }
}

/// Create an incident using retry and consistent logging. Honors dry-run via `reporting_enabled`.
pub async fn create_with_retry(
    client: &IncidentClient,
    reporting_enabled: bool,
    payload: &NewIncident,
) -> Result<String> {
    if reporting_enabled {
        let id = retry_op(|| async { client.create_incident(payload).await }).await?;
        info!(
            incident_id = %id,
            name = %payload.name,
            message = %payload.message,
            status = ?payload.status,
            components = ?payload.components,
            "Created incident"
        );
        Ok(id)
    } else {
        let synthetic_id = format!("dryrun:{}", Utc::now().timestamp_millis());
        warn!(
            incident_id = %synthetic_id,
            name = %payload.name,
            message = %payload.message,
            status = ?payload.status,
            components = ?payload.components,
            "Instatus monitors disabled - would create incident"
        );
        Ok(synthetic_id)
    }
}

/// Resolve an incident using retry and consistent logging. Honors dry-run via `reporting_enabled`.
pub async fn resolve_with_retry(
    client: &IncidentClient,
    reporting_enabled: bool,
    id: &str,
    payload: &ResolveIncident,
) -> Result<()> {
    debug!(%id, "Closing incident");

    if !reporting_enabled {
        info!(%id, components = ?payload.components, "Instatus monitors disabled - would resolve incident");
        return Ok(());
    }

    match retry_op(|| async { client.resolve_incident(id, payload).await }).await {
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
