#![allow(missing_docs)]

use serde_json;

pub use messages::TaikoEvent;

// Placeholder for publishing an event to NATS JetStream
pub async fn publish_event(client: &async_nats::Client, event: &TaikoEvent) -> eyre::Result<()> {
    let js = async_nats::jetstream::from_client(client.clone());
    let stream = js
        .get_or_create_stream(async_nats::jetstream::stream::Config {
            name: "taiko_events".to_string(),
            subjects: vec!["taiko.events".to_string()],
            ..Default::default()
        })
        .await?;
    let payload = serde_json::to_vec(event)?;
    stream.publish("taiko.events", payload.into()).await?;
    Ok(())
}

// Placeholder for subscribing to events from NATS JetStream
pub async fn subscribe_to_events(_client: &async_nats::Client) -> eyre::Result<()> {
    unimplemented!("subscribe_to_events not yet implemented");
}
