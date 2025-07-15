#![allow(missing_docs)]

pub use messages::TaikoEvent;

// Placeholder for publishing an event to NATS JetStream
pub async fn publish_event(
    js: &async_nats::jetstream::Context,
    event: &TaikoEvent,
) -> eyre::Result<()> {
    let payload = serde_json::to_vec(event)?;
    js.publish("taiko.events", payload.into()).await?;
    Ok(())
}

// Placeholder for subscribing to events from NATS JetStream
pub async fn subscribe_to_events(_client: &async_nats::Client) -> eyre::Result<()> {
    unimplemented!("subscribe_to_events not needed - processor handles NATS directly");
}
