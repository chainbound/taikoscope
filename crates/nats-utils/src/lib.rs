#![allow(missing_docs)]

pub use messages::TaikoEvent;

// Placeholder for publishing an event to NATS JetStream
pub async fn publish_event(_client: &async_nats::Client, _event: &TaikoEvent) -> eyre::Result<()> {
    unimplemented!("publish_event not yet implemented");
}

// Placeholder for subscribing to events from NATS JetStream
pub async fn subscribe_to_events(_client: &async_nats::Client) -> eyre::Result<()> {
    unimplemented!("subscribe_to_events not yet implemented");
}
