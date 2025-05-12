use crate::client::{Client as IncidentClient, ComponentStatus, NewIncident, ResolveIncident};
use chrono::{Duration as ChronoDuration, Utc};
use clickhouse::ClickhouseClient;
use tokio::time::Duration as TokioDuration;
use tracing::{error, info};

/// Monitors `ClickHouse` L2 head events and manages Instatus incidents.
/// Polls `ClickHouse` every `interval` seconds; if no L2 head event for `threshold` seconds, it
/// creates an incident; resolves when events resume.
#[derive(Debug)]
pub struct InstatusMonitor {
    clickhouse: ClickhouseClient,
    incident_client: IncidentClient,
    component_id: String,
    threshold: ChronoDuration,
    interval: TokioDuration,
    active_incident: Option<String>,
}

impl InstatusMonitor {
    /// Creates a new `InstatusMonitor` with a 30s threshold and interval.
    pub fn new(
        clickhouse: ClickhouseClient,
        incident_client: IncidentClient,
        component_id: String,
    ) -> Self {
        Self {
            clickhouse,
            incident_client,
            component_id,
            threshold: ChronoDuration::seconds(30),
            interval: TokioDuration::from_secs(30),
            active_incident: None,
        }
    }

    /// Spawns the monitor on the Tokio runtime.
    pub fn spawn(self) {
        tokio::spawn(async move { self.run().await });
    }

    async fn run(mut self) {
        loop {
            tokio::time::sleep(self.interval).await;
            match self.clickhouse.get_last_l2_head_time().await {
                Ok(Some(last)) => {
                    let age = Utc::now().signed_duration_since(last);
                    if self.active_incident.is_none() && age > self.threshold {
                        let payload = NewIncident {
                            name: "No L2 head events - Possible Outage".to_string(),
                            message: format!(
                                "No L2 head event for {}s",
                                self.threshold.num_seconds()
                            ),
                            status: "INVESTIGATING".into(),
                            components: vec![self.component_id.clone()],
                            statuses: vec![ComponentStatus {
                                id: self.component_id.clone(),
                                status: "MAJOROUTAGE".into(),
                            }],
                            notify: true,
                            started: Some(last.to_rfc3339()),
                        };
                        if let Ok(id) = self.incident_client.create_incident(payload).await {
                            info!("Created Instatus incident {}", id);
                            self.active_incident = Some(id);
                        }
                    } else if let Some(ref id) = self.active_incident {
                        if age <= self.threshold {
                            let payload = ResolveIncident {
                                message: "L2 head events have resumed.".into(),
                                status: "RESOLVED".into(),
                                components: vec![self.component_id.clone()],
                                statuses: vec![ComponentStatus {
                                    id: self.component_id.clone(),
                                    status: "OPERATIONAL".into(),
                                }],
                                notify: true,
                            };
                            if matches!(
                                self.incident_client.resolve_incident(id, payload).await,
                                Ok(())
                            ) {
                                info!("Resolved Instatus incident {}", id);
                                self.active_incident = None;
                            }
                        }
                    }
                }
                Ok(None) => {
                    // no L2 head event yet, skip
                }
                Err(e) => {
                    error!(err = %e, "Error polling last L2 head time");
                }
            }
        }
    }
}
