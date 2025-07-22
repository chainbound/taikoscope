use super::*;
use crate::{base_monitor::Monitor, client::Client as IncidentClient};
use chrono::{Duration as ChronoDuration, Utc};
use clickhouse::ClickhouseReader as ClickhouseInternalClient;
use mockito::{Matcher, Server, ServerGuard};
use std::time::Duration;
use url::Url;

// Helper to create a ClickhouseClient for tests
fn mock_clickhouse_client() -> (ClickhouseInternalClient, ServerGuard) {
    let server = mockito::Server::new();
    let url = Url::parse(&server.url()).unwrap();
    let client = ClickhouseInternalClient::new(
        url,
        "test_db".to_owned(),
        "user".to_owned(),
        "pass".to_owned(),
    )
    .unwrap();
    (client, server)
}

// Helper to create an IncidentClient for tests
fn mock_incident_client() -> (IncidentClient, ServerGuard) {
    let server = mockito::Server::new();
    let url = Url::parse(&server.url()).unwrap();
    let client =
        IncidentClient::with_base_url("test_api_key".to_owned(), "test_page_id".to_owned(), url);
    (client, server)
}

async fn mock_clickhouse_client_async() -> (ClickhouseInternalClient, ServerGuard) {
    let server = Server::new_async().await;
    let url = Url::parse(&server.url()).unwrap();
    let client = ClickhouseInternalClient::new(
        url,
        "test_db".to_owned(),
        "user".to_owned(),
        "pass".to_owned(),
    )
    .unwrap();
    (client, server)
}

#[tokio::test]
async fn instatus_monitor_create_and_resolve_incident() {
    let (ch_client, _ch_server) = mock_clickhouse_client_async().await;
    let mut server = Server::new_async().await;

    let post_mock = server
        .mock("POST", "/v1/test_page_id/incidents")
        .match_header("authorization", "Bearer test_api_key")
        .match_header("content-type", "application/json")
        .with_status(200)
        .with_body(r#"{"id":"inc1"}"#)
        .create_async()
        .await;

    let put_mock = server
        .mock("PUT", "/v1/test_page_id/incidents/inc1")
        .match_header("authorization", "Bearer test_api_key")
        .match_header("content-type", "application/json")
        .with_status(200)
        .with_body("{}")
        .create_async()
        .await;

    let get_mock = server
        .mock("GET", "/v1/test_page_id/incidents")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_body("[]")
        .create_async()
        .await;

    let incident_client = IncidentClient::with_base_url(
        "test_api_key".into(),
        "test_page_id".into(),
        server.url().parse().unwrap(),
    );

    let monitor = InstatusMonitor::new(
        ch_client,
        incident_client,
        "comp1".to_owned(),
        Duration::from_secs(60),
        Duration::from_secs(1),
    );

    let id = monitor.create_incident(&()).await.unwrap();
    assert_eq!(id, "inc1");

    monitor.resolve_incident(&id).await.unwrap();

    post_mock.assert_async().await;
    put_mock.assert_async().await;
    get_mock.assert_async().await;
}

#[tokio::test]
async fn instatus_l1_monitor_create_and_resolve_incident() {
    let (ch_client, _ch_server) = mock_clickhouse_client_async().await;
    let mut server = Server::new_async().await;

    let post_mock = server
        .mock("POST", "/v1/test_page_id/incidents")
        .match_header("authorization", "Bearer test_api_key")
        .match_header("content-type", "application/json")
        .with_status(200)
        .with_body(r#"{"id":"inc1"}"#)
        .create_async()
        .await;

    let put_mock = server
        .mock("PUT", "/v1/test_page_id/incidents/inc1")
        .match_header("authorization", "Bearer test_api_key")
        .match_header("content-type", "application/json")
        .with_status(200)
        .with_body("{}")
        .create_async()
        .await;

    let incident_client = IncidentClient::with_base_url(
        "test_api_key".into(),
        "test_page_id".into(),
        server.url().parse().unwrap(),
    );

    let monitor = InstatusL1Monitor::new(
        ch_client,
        incident_client,
        "comp1".to_owned(),
        Duration::from_secs(60),
        Duration::from_secs(1),
    );

    let id = monitor.create_incident(&()).await.unwrap();
    assert_eq!(id, "inc1");

    monitor.resolve_incident(&id).await.unwrap();

    post_mock.assert_async().await;
    put_mock.assert_async().await;
}

#[tokio::test]
async fn instatus_monitor_handle_opens_and_resolves_incident() {
    let (ch_client, _ch_server) = mock_clickhouse_client_async().await;
    let mut server = Server::new_async().await;

    let post_mock = server
        .mock("POST", "/v1/test_page_id/incidents")
        .match_header("authorization", "Bearer test_api_key")
        .match_header("content-type", "application/json")
        .with_status(200)
        .with_body(r#"{"id":"inc1"}"#)
        .create_async()
        .await;

    let put_mock = server
        .mock("PUT", "/v1/test_page_id/incidents/inc1")
        .match_header("authorization", "Bearer test_api_key")
        .match_header("content-type", "application/json")
        .with_status(200)
        .with_body("{}")
        .create_async()
        .await;

    // Mock for incident_exists check
    let incident_exists_mock = server
        .mock("GET", "/v1/test_page_id/incidents/inc1")
        .match_header("authorization", "Bearer test_api_key")
        .with_status(200)
        .with_body(r#"{"id":"inc1"}"#)
        .create_async()
        .await;

    let get_mock = server
        .mock("GET", "/v1/test_page_id/incidents")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_body("[]")
        .create_async()
        .await;

    let incident_client = IncidentClient::with_base_url(
        "test_api_key".into(),
        "test_page_id".into(),
        server.url().parse().unwrap(),
    );

    let mut monitor = InstatusMonitor::new(
        ch_client,
        incident_client,
        "comp1".to_owned(),
        Duration::from_secs(60),
        Duration::from_secs(1),
    );

    let outdated = Utc::now() - ChronoDuration::seconds(120);
    monitor.handle(outdated).await.unwrap();
    assert_eq!(monitor.base.active_incidents.get(&()), Some(&"inc1".to_owned()));

    monitor.handle(Utc::now()).await.unwrap();
    assert!(monitor.base.active_incidents.is_empty());

    put_mock.assert_async().await;
    post_mock.assert_async().await;
    incident_exists_mock.assert_async().await;
    get_mock.assert_async().await;
}

#[tokio::test]
async fn instatus_monitor_does_not_duplicate_incident() {
    let (ch_client, _ch_server) = mock_clickhouse_client_async().await;
    let mut server = Server::new_async().await;

    let list = serde_json::json!([
        {"id": "inc1", "components": [{"id": "comp1", "name": "comp1", "status": "MAJOROUTAGE"}]}
    ])
    .to_string();

    let get_mock = server
        .mock("GET", "/v1/test_page_id/incidents")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_body(list)
        .create_async()
        .await;

    let post_mock =
        server.mock("POST", "/v1/test_page_id/incidents").expect(0).create_async().await;

    let incident_client = IncidentClient::with_base_url(
        "test_api_key".into(),
        "test_page_id".into(),
        server.url().parse().unwrap(),
    );

    let mut monitor = InstatusMonitor::new(
        ch_client,
        incident_client,
        "comp1".to_owned(),
        Duration::from_secs(60),
        Duration::from_secs(1),
    );

    let outdated = Utc::now() - ChronoDuration::seconds(120);
    monitor.handle(outdated).await.unwrap();
    assert_eq!(monitor.base.active_incidents.get(&()), Some(&"inc1".to_owned()));

    get_mock.assert_async().await;
    post_mock.assert_async().await;
}

#[test]
fn filter_new_batches_only_returns_untracked() {
    let (ch_client, _ch_server) = mock_clickhouse_client();
    let (incident_client, _incident_server) = mock_incident_client();
    let mut monitor = BatchProofTimeoutMonitor::new(
        ch_client,
        incident_client,
        "comp".to_owned(),
        Duration::from_secs(1),
        Duration::from_secs(1),
    );
    monitor.base.active_incidents.insert((1, 1), "id".to_owned());
    let now = Utc::now();
    let batches = vec![(1, 1, now), (2, 2, now)];
    let filtered = monitor.filter_new_batches(&batches);
    assert_eq!(filtered, vec![(2, 2, now)]);
}

#[test]
fn catch_all_only_true_only_for_catch_all() {
    let (ch_client, _ch_server) = mock_clickhouse_client();
    let (incident_client, _incident_server) = mock_incident_client();
    let mut monitor = BatchProofTimeoutMonitor::new(
        ch_client,
        incident_client,
        "comp".to_owned(),
        Duration::from_secs(1),
        Duration::from_secs(1),
    );
    assert!(!monitor.catch_all_only());
    monitor.base.active_incidents.insert((0, 0), "id".to_owned());
    assert!(monitor.catch_all_only());
    monitor.base.active_incidents.insert((1, 1), "other".to_owned());
    assert!(!monitor.catch_all_only());
}
