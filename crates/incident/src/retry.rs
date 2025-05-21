use eyre::Report;
use primitives::retries::retry_with_backoff_if;
use reqwest::{Error as ReqwestError, StatusCode};

/// Determine if an error returned by reqwest/eyre is retryable.
fn is_retryable(err: &Report) -> bool {
    if let Some(req_err) = err.downcast_ref::<ReqwestError>() {
        if req_err.is_timeout() || req_err.is_connect() {
            return true;
        }
        if let Some(status) = req_err.status() {
            return status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS;
        }
    }
    false
}

/// Retry the provided async operation with exponential backoff if the returned
/// error is considered retryable.
pub(crate) async fn retry_op<F, Fut, T>(op: F) -> eyre::Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = eyre::Result<T>>,
{
    retry_with_backoff_if(op, is_retryable).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use reqwest::Client;

    #[tokio::test]
    async fn retries_when_error_is_retryable() {
        let mut server = Server::new_async().await;
        let mock = server.mock("GET", "/").with_status(500).expect_at_least(2).create_async().await;

        let client = Client::new();
        let url = server.url();
        let result = retry_op(|| async {
            let resp = client.get(url.clone()).send().await?;
            resp.error_for_status()?;
            Ok::<(), eyre::Report>(())
        })
        .await;

        assert!(result.is_err());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn does_not_retry_for_non_retryable_error() {
        let mut server = Server::new_async().await;
        let mock = server.mock("GET", "/").with_status(400).expect(1).create_async().await;

        let client = Client::new();
        let url = server.url();
        let result = retry_op(|| async {
            let resp = client.get(url.clone()).send().await?;
            resp.error_for_status()?;
            Ok::<(), eyre::Report>(())
        })
        .await;

        assert!(result.is_err());
        mock.assert_async().await;
    }
}
