use std::time::Duration;

use alloy::{
    providers::WsConnect,
    pubsub::{ConnectionHandle, PubSubConnect},
    transports::{
        RpcError, TransportError, TransportErrorKind, TransportResult,
        http::reqwest::Url,
        layers::{RetryBackoffLayer, RetryPolicy},
    },
};
use alloy_json_rpc::ErrorPayload;
use serde::Deserialize;
use tokio_retry::{Retry, RetryIf, strategy::ExponentialBackoff};
use tracing::warn;

/// The default maximum number of retries for a transport error.
///
/// With a `DEFAULT_INITIAL_BACKOFF_MS` of 1ms we can do 9 retries in ~500ms:
const DEFAULT_MAX_RETRIES: u32 = 9;

/// The default initial backoff time in milliseconds for a transport error.
const DEFAULT_INITIAL_BACKOFF_MS: u64 = 1;

/// The default [`RetryBackoffLayer`] for a transport error.
pub const DEFAULT_RETRY_LAYER: RetryBackoffLayer<RateLimitConnRefusedRetryPolicy> =
    RetryBackoffLayer::new_with_policy(
        DEFAULT_MAX_RETRIES,
        DEFAULT_INITIAL_BACKOFF_MS,
        100,
        RateLimitConnRefusedRetryPolicy,
    );

/// A retry strategy trait.
pub trait Strategy: Iterator<Item = Duration> + Clone + Send + Sync + 'static {}

/// Implement the Strategy trait for any type that is an iterator of Durations (i.e. all backoffs
/// exported by `tokio_retry`)
impl<T> Strategy for T where T: Iterator<Item = Duration> + Clone + Send + Sync + 'static {}

/// A [`WsConnect`] wrapper with retries on connection failures.
/// Uses exponential backoff.
#[derive(Debug, Clone)]
pub struct RetryWsConnect<S: Strategy> {
    inner: WsConnect,
    strategy: S,
}

impl RetryWsConnect<ExponentialBackoff> {
    /// Create a new `RetryWsConnect` with the [`DEFAULT_INITIAL_BACKOFF_MS`] strategy.
    #[inline]
    pub fn from_url<U: Into<Url>>(ws_url: U) -> Self {
        Self {
            inner: WsConnect::new(ws_url.into()),
            strategy: ExponentialBackoff::from_millis(DEFAULT_INITIAL_BACKOFF_MS),
        }
    }
}

impl<S: Strategy> PubSubConnect for RetryWsConnect<S> {
    fn is_local(&self) -> bool {
        self.inner.is_local()
    }

    async fn connect(&self) -> TransportResult<ConnectionHandle> {
        self.inner.connect().await
    }

    fn try_reconnect(
        &self,
    ) -> alloy::transports::impl_future!(<Output = TransportResult<ConnectionHandle>>) {
        warn!(url = ?self.inner.url(),"Retrying connection to websocket provider");
        Retry::spawn(self.strategy.clone(), || self.inner.try_reconnect())
    }
}

/// Extension trait to implement methods for [`RpcError<TransportErrorKind, E>`].
///
/// Ported from Alloy because it is private to its crate.
/// Reference: <https://github.com/alloy-rs/alloy/blob/a3d521e18fe335f5762be03656a3470f5f6331d8/crates/transport/src/error.rs#L126>
pub(crate) trait RpcErrorExt {
    /// Analyzes whether to retry the request depending on the error.
    fn is_retryable(&self) -> bool;

    /// Fetches the backoff hint from the error message if present
    fn backoff_hint(&self) -> Option<std::time::Duration>;
}

impl RpcErrorExt for RpcError<TransportErrorKind> {
    fn is_retryable(&self) -> bool {
        match self {
            // There was a transport-level error. This is either a non-retryable error,
            // or a server error that should be retried.
            Self::Transport(err) => err.is_retry_err(),
            Self::DeserError { text, .. } => {
                if let Ok(resp) = serde_json::from_str::<ErrorPayload>(text) {
                    return resp.is_retry_err();
                }

                // some providers send invalid JSON RPC in the error case (no `id:u64`), but the
                // text should be a `JsonRpcError`
                #[derive(Deserialize)]
                struct Resp {
                    error: ErrorPayload,
                }

                if let Ok(resp) = serde_json::from_str::<Resp>(text) {
                    return resp.error.is_retry_err();
                }

                false
            }
            Self::ErrorResp(err) => err.is_retry_err(),
            Self::NullResp => true,
            // The transport could not serialize the error itself. The request was malformed from
            // the start.
            #[allow(clippy::match_same_arms)]
            Self::SerError(_) => false,
            _ => false,
        }
    }

    fn backoff_hint(&self) -> Option<std::time::Duration> {
        if let Self::ErrorResp(resp) = self {
            let data = resp.try_data_as::<serde_json::Value>();
            if let Some(Ok(data)) = data {
                // if daily rate limit exceeded, infura returns the requested backoff in the error
                // response
                let backoff_seconds = &data["rate"]["backoff_seconds"];
                // infura rate limit error
                if let Some(seconds) = backoff_seconds.as_u64() {
                    return Some(std::time::Duration::from_secs(seconds));
                }
                if let Some(seconds) = backoff_seconds.as_f64() {
                    return Some(std::time::Duration::from_secs(seconds as u64 + 1));
                }
            }
        }
        None
    }
}

/// A retry policy that retries also on "connection refused" errors.
#[derive(Debug, Clone)]
pub struct RateLimitConnRefusedRetryPolicy;

impl RetryPolicy for RateLimitConnRefusedRetryPolicy {
    fn should_retry(&self, error: &TransportError) -> bool {
        error.is_retryable() || is_connection_refused(error)
    }

    fn backoff_hint(&self, error: &TransportError) -> Option<Duration> {
        error.backoff_hint()
    }
}

/// Checks whether the error message contains "connection refused".
#[inline]
pub fn is_connection_refused<S: ToString>(e: S) -> bool {
    e.to_string().to_lowercase().contains("connection refused")
}

/// Retry the provided async operation using [`ExponentialBackoff`].
///
/// Retries are attempted as long as the provided `condition` returns `true` for
/// the error produced by the operation. The backoff uses the same
/// `DEFAULT_MAX_RETRIES` and `DEFAULT_INITIAL_BACKOFF_MS` constants as the
/// websocket retry layer.
pub async fn retry_with_backoff_if<F, Fut, T, E, C>(op: F, condition: C) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    C: Fn(&E) -> bool,
{
    let strategy = ExponentialBackoff::from_millis(DEFAULT_INITIAL_BACKOFF_MS)
        .take(DEFAULT_MAX_RETRIES as usize);
    RetryIf::spawn(strategy, op, condition).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn is_retryable_on_known_rate_limit_errors() {
        let err = "{\"code\":-32007,\"message\":\"100/second request limit reached - reduce calls per second or upgrade your account at quicknode.com\"}";
        let err = serde_json::from_str::<ErrorPayload>(err).unwrap();
        assert!(RpcError::<TransportErrorKind>::ErrorResp(err).is_retryable());
    }

    #[test]
    fn is_retryable_on_http_429() {
        let err = r#"{"code":429,"event":-33200,"message":"Too Many Requests","details":"limit"}"#;
        let err = serde_json::from_str::<ErrorPayload>(err).unwrap();
        assert!(RpcError::<TransportErrorKind>::ErrorResp(err).is_retryable());
    }

    #[test]
    fn backoff_hint_parses_integer_seconds() {
        let json =
            r#"{"code":429,"message":"Too Many Requests","data":{"rate":{"backoff_seconds":7}}}"#;
        let payload = serde_json::from_str::<ErrorPayload>(json).unwrap();
        let err = RpcError::<TransportErrorKind>::ErrorResp(payload);
        assert_eq!(err.backoff_hint(), Some(Duration::from_secs(7)));
    }

    #[test]
    fn backoff_hint_rounds_up_fractional_seconds() {
        let json =
            r#"{"code":429,"message":"Too Many Requests","data":{"rate":{"backoff_seconds":2.3}}}"#;
        let payload = serde_json::from_str::<ErrorPayload>(json).unwrap();
        let err = RpcError::<TransportErrorKind>::ErrorResp(payload);
        assert_eq!(err.backoff_hint(), Some(Duration::from_secs(3)));
    }

    #[test]
    fn connection_refused_detection_is_case_insensitive() {
        assert!(is_connection_refused("Connection Refused"));
        assert!(is_connection_refused("connection refused"));
        assert!(!is_connection_refused("other error"));
    }

    #[test]
    fn is_retryable_on_null_response() {
        assert!(RpcError::<TransportErrorKind>::NullResp.is_retryable());
    }
}
