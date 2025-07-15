#![allow(missing_docs)]

pub use messages::TaikoEvent;

// Placeholder for publishing an event to NATS JetStream
pub async fn publish_event(client: &async_nats::Client, event: &TaikoEvent) -> eyre::Result<()> {
    let js = async_nats::jetstream::new(client.clone());
    let _stream = js
        .get_or_create_stream(async_nats::jetstream::stream::Config {
            name: "taiko".to_owned(),
            subjects: vec!["taiko.events".to_owned()],
            ..Default::default()
        })
        .await?;
    let payload = serde_json::to_vec(event)?;
    js.publish("taiko.events", payload.into()).await?;
    Ok(())
}

// Placeholder for subscribing to events from NATS JetStream
pub async fn subscribe_to_events(_client: &async_nats::Client) -> eyre::Result<()> {
    unimplemented!("subscribe_to_events not needed - processor handles NATS directly");
}
