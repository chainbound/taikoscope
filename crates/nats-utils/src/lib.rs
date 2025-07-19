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

/// Publish event with exactly-once semantics using message ID headers
pub async fn publish_event_with_dedup(
    js: &async_nats::jetstream::Context,
    event: &TaikoEvent,
) -> eyre::Result<async_nats::jetstream::publish::PublishAck> {
    let dedup_id = event.dedup_id();
    let payload = serde_json::to_vec(event)?;

    // Create headers with message ID for deduplication
    let mut headers = async_nats::HeaderMap::new();
    headers.insert("Msg-Id", dedup_id.as_str());
    headers.insert("Content-Type", "application/json");

    // Create publish request with message ID header for deduplication
    let publish_ack =
        js.publish_with_headers("taiko.events", headers, payload.into()).await?.await?;

    tracing::debug!(
        dedup_id = %dedup_id,
        sequence = publish_ack.sequence,
        "Published event with deduplication ID"
    );

    Ok(publish_ack)
}

/// Publish event with exactly-once semantics and retry logic
pub async fn publish_event_with_retry(
    js: &async_nats::jetstream::Context,
    event: &TaikoEvent,
    max_retries: u32,
) -> eyre::Result<async_nats::jetstream::publish::PublishAck> {
    let dedup_id = event.dedup_id();
    let mut retries = 0;

    loop {
        match publish_event_with_dedup(js, event).await {
            Ok(ack) => {
                if retries > 0 {
                    tracing::info!(
                        dedup_id = %dedup_id,
                        retries = retries,
                        "Successfully published event after retries"
                    );
                }
                return Ok(ack);
            }
            Err(e) => {
                if retries >= max_retries {
                    tracing::error!(
                        dedup_id = %dedup_id,
                        retries = retries,
                        error = %e,
                        "Failed to publish event after max retries"
                    );
                    return Err(e);
                }

                retries += 1;
                let delay = std::time::Duration::from_millis(100 * (1 << retries));

                tracing::warn!(
                    dedup_id = %dedup_id,
                    retry = retries,
                    delay_ms = delay.as_millis(),
                    error = %e,
                    "Retrying publish after error"
                );

                tokio::time::sleep(delay).await;
            }
        }
    }
}

// Placeholder for subscribing to events from NATS JetStream
pub async fn subscribe_to_events(_client: &async_nats::Client) -> eyre::Result<()> {
    unimplemented!("subscribe_to_events not needed - processor handles NATS directly");
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::B256;
    use primitives::headers::L1Header;

    fn create_test_event() -> TaikoEvent {
        TaikoEvent::L1Header(L1Header {
            number: 12345,
            hash: B256::from_slice(&[0u8; 32]),
            slot: 1,
            timestamp: 1234567890,
        })
    }

    #[tokio::test]
    async fn test_publish_functions_exist() -> eyre::Result<()> {
        // Just test that our publish functions compile and can be called
        // without requiring a running NATS server
        let event = create_test_event();
        let dedup_id = event.dedup_id();

        // Test that dedup_id generation works
        assert!(!dedup_id.is_empty());
        assert!(dedup_id.contains("l1_header"));

        Ok(())
    }
}
