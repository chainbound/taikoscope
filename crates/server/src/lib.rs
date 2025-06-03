//! Helper utilities to launch the Taikoscope API server.

use std::{net::SocketAddr, sync::Arc, time::Duration};

use api::{self, ApiState};
use axum::{
    Router,
    http::{HeaderValue, Method},
};
use clickhouse_lib::ClickhouseReader;
use eyre::Result;
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::{Level, info};

/// Allowed CORS origins for dashboard requests.
const ALLOWED_ORIGINS: &[&str] = &["https://taikoscope.xyz", "https://www.taikoscope.xyz"];

/// Version prefix for all API routes.
pub const API_VERSION: &str = "v1";

/// Build the API router with CORS and tracing layers.
pub fn router(state: ApiState, extra_origins: Vec<String>) -> Router {
    let extra = Arc::new(extra_origins);
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate({
            let extra = Arc::clone(&extra);
            move |origin: &HeaderValue, _| match origin.to_str() {
                Ok(origin) => {
                    ALLOWED_ORIGINS.contains(&origin) ||
                        origin.ends_with(".vercel.app") ||
                        extra.iter().any(|o| o == origin)
                }
                Err(_) => false,
            }
        }))
        .allow_methods([Method::GET])
        .allow_headers(Any);
    let trace = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO));

    Router::new().nest(&format!("/{API_VERSION}"), api::router(state)).layer(cors).layer(trace)
}

/// Run the API server on the given address.
pub async fn run(
    addr: SocketAddr,
    client: ClickhouseReader,
    extra_origins: Vec<String>,
    max_requests: u64,
    rate_period: Duration,
) -> Result<()> {
    let state = ApiState::new(client, max_requests, rate_period);
    let app = router(state, extra_origins);

    info!("Starting API server on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use api::{ApiState, DEFAULT_MAX_REQUESTS, DEFAULT_RATE_PERIOD};
    use axum::{
        body::{self, Body},
        http::{Request, StatusCode},
    };
    use chrono::Utc;
    use clickhouse::{
        Row,
        test::{Mock, handlers},
    };
    use serde::Serialize;
    use serde_json::{Value, json};
    use tower::util::ServiceExt;
    use url::Url;

    #[derive(Serialize, Row)]
    struct MaxRow {
        block_ts: u64,
    }

    fn build_app(mock_url: &str, extra: Vec<String>) -> Router {
        let url = Url::parse(mock_url).unwrap();
        let client =
            ClickhouseReader::new(url, "db".to_owned(), "user".into(), "pass".into()).unwrap();
        let state = ApiState::new(client, DEFAULT_MAX_REQUESTS, DEFAULT_RATE_PERIOD);
        router(state, extra)
    }

    async fn send_request(app: Router, origin: &str) -> (StatusCode, Value, Option<String>) {
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/{API_VERSION}/l2-head"))
                    .header("Origin", origin)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let cors = response
            .headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok())
            .map(ToOwned::to_owned);
        let bytes = body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        (status, body, cors)
    }

    #[tokio::test]
    async fn allows_default_origin() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![MaxRow { block_ts: 1 }]));
        let app = build_app(mock.url(), vec![]);
        let (status, body, cors) = send_request(app, "https://taikoscope.xyz").await;
        let expected = json!({
            "last_l2_head_time": Utc.timestamp_opt(1, 0).single().unwrap().to_rfc3339()
        });
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, expected);
        assert_eq!(cors.as_deref(), Some("https://taikoscope.xyz"));
    }

    #[tokio::test]
    async fn allows_extra_origin() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![MaxRow { block_ts: 1 }]));
        let app = build_app(mock.url(), vec!["https://example.com".to_string()]);
        let (status, _, cors) = send_request(app, "https://example.com").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(cors.as_deref(), Some("https://example.com"));
    }

    #[tokio::test]
    async fn denies_other_origin() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![MaxRow { block_ts: 1 }]));
        let app = build_app(mock.url(), vec![]);
        let (status, _, cors) = send_request(app, "https://notallowed.com").await;
        assert_eq!(status, StatusCode::OK);
        assert!(cors.is_none());
    }
}
