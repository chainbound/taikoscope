use eyre::Report;
pub use network::http_retry::retry_op;
use reqwest::{Error as ReqwestError, StatusCode};

/// Determine if an error returned by reqwest/eyre is retryable for the Instatus API.
///
/// `429 Too Many Requests` responses are treated as non-retryable since we send
/// very few requests and hitting this limit likely indicates a bug causing an
/// endless loop.
///
/// `PAGE_MISMATCH` errors are also non-retryable as they indicate a configuration
/// issue where the incident belongs to a different page.
pub fn is_retryable(err: &Report) -> bool {
    // Check if this is a PAGE_MISMATCH error (configuration issue)
    let err_msg = format!("{}", err);
    if err_msg.contains("PAGE_MISMATCH") {
        return false;
    }

    if let Some(req_err) = err.downcast_ref::<ReqwestError>() {
        if req_err.is_timeout() || req_err.is_connect() {
            return true;
        }
        if let Some(status) = req_err.status() {
            return status.is_server_error() && status != StatusCode::TOO_MANY_REQUESTS;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use eyre::Report;
    use mockito::Server;
    use reqwest::Client;

    #[tokio::test]
    async fn is_retryable_returns_false_for_http_429() {
        let mut server = Server::new_async().await;
        let _mock = server.mock("GET", "/").with_status(429).create_async().await;

        let client = Client::new();
        let url = server.url();
        let err = client.get(url).send().await.unwrap().error_for_status().unwrap_err();
        assert!(!super::is_retryable(&Report::from(err)));
    }
}
