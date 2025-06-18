//! Server-Sent Events endpoints

use crate::state::ApiState;
use async_stream::stream;
use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use futures::stream::Stream;
use std::{convert::Infallible, time::Duration as StdDuration};

pub async fn sse_l2_head(
    State(state): State<ApiState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut last = state.client.get_last_l2_block_number().await.ok().flatten().unwrap_or(0);
    let mut error_count = 0;
    let mut last_successful_fetch = std::time::Instant::now();

    let stream = stream! {
        // send current head immediately
        yield Ok(Event::default().data(last.to_string()));

        loop {
            // Add timeout to the database query to prevent long-running requests
            let fetch_result = tokio::time::timeout(
                StdDuration::from_secs(30), // 30 second timeout for database queries
                state.client.get_last_l2_block_number()
            ).await;

            match fetch_result {
                Ok(Ok(Some(num))) if num != last => {
                    last = num;
                    error_count = 0; // Reset error count on success
                    last_successful_fetch = std::time::Instant::now();
                    yield Ok(Event::default().data(num.to_string()));
                }
                Ok(Ok(_)) => {
                    // No change in block number, reset error count
                    error_count = 0;
                    last_successful_fetch = std::time::Instant::now();
                }
                Ok(Err(e)) => {
                    error_count += 1;
                    tracing::error!("Failed to fetch L2 head block (attempt {}): {}", error_count, e);

                    // If we've had many consecutive errors, send the last known value
                    if error_count >= 5 && last_successful_fetch.elapsed() > StdDuration::from_secs(60) {
                        tracing::warn!("L2 head SSE: Using cached value due to persistent database errors");
                        yield Ok(Event::default().data(last.to_string()));
                    }
                }
                Err(_timeout) => {
                    error_count += 1;
                    tracing::error!("Timeout fetching L2 head block (attempt {})", error_count);

                    // On timeout, send cached value to keep connection alive
                    if error_count >= 3 {
                        yield Ok(Event::default().data(last.to_string()));
                    }
                }
            }

            // Adaptive sleep interval based on error state
            let sleep_duration = if error_count > 0 {
                // Back off when there are errors
                StdDuration::from_secs((error_count as u64).min(10))
            } else {
                StdDuration::from_secs(1)
            };

            tokio::time::sleep(sleep_duration).await;
        }
    };

    // More aggressive keep-alive settings to prevent proxy timeouts
    let keep_alive = KeepAlive::new().interval(StdDuration::from_secs(15)).text("keepalive");

    Sse::new(stream).keep_alive(keep_alive)
}

pub async fn sse_l1_head(
    State(state): State<ApiState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut last = state.client.get_last_l1_block_number().await.ok().flatten().unwrap_or(0);
    let mut error_count = 0;
    let mut last_successful_fetch = std::time::Instant::now();

    let stream = stream! {
        // send current head immediately
        yield Ok(Event::default().data(last.to_string()));

        loop {
            // Add timeout to the database query to prevent long-running requests
            let fetch_result = tokio::time::timeout(
                StdDuration::from_secs(30), // 30 second timeout for database queries
                state.client.get_last_l1_block_number()
            ).await;

            match fetch_result {
                Ok(Ok(Some(num))) if num != last => {
                    last = num;
                    error_count = 0; // Reset error count on success
                    last_successful_fetch = std::time::Instant::now();
                    yield Ok(Event::default().data(num.to_string()));
                }
                Ok(Ok(_)) => {
                    // No change in block number, reset error count
                    error_count = 0;
                    last_successful_fetch = std::time::Instant::now();
                }
                Ok(Err(e)) => {
                    error_count += 1;
                    tracing::error!("Failed to fetch L1 head block (attempt {}): {}", error_count, e);

                    // If we've had many consecutive errors, send the last known value
                    if error_count >= 5 && last_successful_fetch.elapsed() > StdDuration::from_secs(60) {
                        tracing::warn!("L1 head SSE: Using cached value due to persistent database errors");
                        yield Ok(Event::default().data(last.to_string()));
                    }
                }
                Err(_timeout) => {
                    error_count += 1;
                    tracing::error!("Timeout fetching L1 head block (attempt {})", error_count);

                    // On timeout, send cached value to keep connection alive
                    if error_count >= 3 {
                        yield Ok(Event::default().data(last.to_string()));
                    }
                }
            }

            // Adaptive sleep interval based on error state
            let sleep_duration = if error_count > 0 {
                // Back off when there are errors
                StdDuration::from_secs((error_count as u64).min(10))
            } else {
                StdDuration::from_secs(1)
            };

            tokio::time::sleep(sleep_duration).await;
        }
    };

    // More aggressive keep-alive settings to prevent proxy timeouts
    let keep_alive = KeepAlive::new().interval(StdDuration::from_secs(15)).text("keepalive");

    Sse::new(stream).keep_alive(keep_alive)
}
