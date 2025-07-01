#![allow(clippy::redundant_pub_crate)]

use eyre::Result;
use tokio::time::{Duration, sleep};

/// Subscribe to a stream using `subscribe_fn` and retry on failure.
///
/// The provided closure should return a future that resolves to the
/// subscription stream. If the call fails, the function logs the error
/// and retries after five seconds.
pub(crate) async fn subscribe_with_retry<F, Fut, T>(mut subscribe_fn: F, name: &str) -> T
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    loop {
        match subscribe_fn().await {
            Ok(stream) => return stream,
            Err(e) => {
                tracing::error!(stream = name, error = %e, "subscribe failed, retrying in 5s");
                sleep(Duration::from_secs(5)).await;
            }
        }
    }
}
