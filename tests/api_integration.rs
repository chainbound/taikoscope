use std::{net::SocketAddr, time::Duration};

use clickhouse::{
    Row,
    test::{Mock, handlers},
};
use reqwest::StatusCode;
use serde::Serialize;
use tokio::{
    net::TcpStream,
    time::{Instant, sleep},
};
use url::Url;

use api::{ApiState, DEFAULT_MAX_REQUESTS, DEFAULT_RATE_PERIOD};
use axum::{extract::connect_info::IntoMakeServiceWithConnectInfo, serve};
use clickhouse_lib::ClickhouseReader;
use server::{API_VERSION, router};
use tokio::net::TcpListener;

#[derive(Serialize, Row)]
struct MaxRow {
    block_ts: u64,
}

async fn spawn_server(client: ClickhouseReader) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let state = ApiState::new(client, DEFAULT_MAX_REQUESTS, DEFAULT_RATE_PERIOD);
    let allowed = config::DEFAULT_ALLOWED_ORIGINS
        .split(',')
        .map(|s| s.to_owned())
        .collect();
    let app = router(state, allowed);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle =
        tokio::spawn(serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()));
    (addr, handle)
}

async fn wait_for_server(addr: SocketAddr) {
    let start = Instant::now();
    loop {
        if TcpStream::connect(addr).await.is_ok() {
            break;
        }
        if start.elapsed() > Duration::from_secs(5) {
            panic!("server did not start in time");
        }
        sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn l2_head_integration() {
    let mock = Mock::new();
    let ts = 42u64;
    mock.add(handlers::provide(vec![MaxRow { block_ts: ts }]));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let resp = reqwest::get(format!("http://{addr}/{API_VERSION}/l2-head")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    let expected = chrono::Utc.timestamp_opt(ts as i64, 0).single().unwrap().to_rfc3339();
    assert_eq!(body, serde_json::json!({ "last_l2_head_time": expected }));

    server.abort();
}

#[tokio::test]
async fn l1_head_integration() {
    let mock = Mock::new();
    let ts = 24u64;
    mock.add(handlers::provide(vec![MaxRow { block_ts: ts }]));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let resp = reqwest::get(format!("http://{addr}/{API_VERSION}/l1-head")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    let expected = chrono::Utc.timestamp_opt(ts as i64, 0).single().unwrap().to_rfc3339();
    assert_eq!(body, serde_json::json!({ "last_l1_head_time": expected }));

    server.abort();
}

#[derive(Serialize, Row)]
struct NumRow {
    number: u64,
}

#[tokio::test]
async fn l2_head_block_integration() {
    let mock = Mock::new();
    mock.add(handlers::provide(vec![NumRow { number: 5 }]));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let resp = reqwest::get(format!("http://{addr}/{API_VERSION}/l2-head-block")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body, serde_json::json!({ "l2_head_block": 5 }));

    server.abort();
}

#[tokio::test]
async fn sse_l2_head_integration() {
    let mock = Mock::new();
    let num = 7u64;
    mock.add(handlers::provide(vec![NumRow { number: num }]));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let resp = reqwest::Client::new()
        .get(format!("http://{addr}/{API_VERSION}/sse/l2-head"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let chunk = resp.chunk().await.unwrap().unwrap();
    let body = String::from_utf8(chunk.to_vec()).unwrap();
    assert_eq!(body, format!("data: {num}\n\n"));

    server.abort();
}

#[tokio::test]
async fn sse_l1_head_integration() {
    let mock = Mock::new();
    let num = 5u64;
    mock.add(handlers::provide(vec![NumRow { number: num }]));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let resp = reqwest::Client::new()
        .get(format!("http://{addr}/{API_VERSION}/sse/l1-head"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let chunk = resp.chunk().await.unwrap().unwrap();
    let body = String::from_utf8(chunk.to_vec()).unwrap();
    assert_eq!(body, format!("data: {num}\n\n"));

    server.abort();
}

#[tokio::test]
async fn sse_l2_head_initial_query_error() {
    let mock = Mock::new();
    mock.add(handlers::failure(clickhouse::test::status::INTERNAL_SERVER_ERROR));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let resp = reqwest::Client::new()
        .get(format!("http://{addr}/{API_VERSION}/sse/l2-head"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["type"], "database-error");

    server.abort();
}

#[tokio::test]
async fn sse_l1_head_initial_query_error() {
    let mock = Mock::new();
    mock.add(handlers::failure(clickhouse::test::status::INTERNAL_SERVER_ERROR));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let resp = reqwest::Client::new()
        .get(format!("http://{addr}/{API_VERSION}/sse/l1-head"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["type"], "database-error");

    server.abort();
}

#[tokio::test]
async fn health_endpoint_unversioned() {
    let mock = Mock::new();
    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    // Test that health endpoint is accessible at unversioned path
    let resp = reqwest::get(format!("http://{addr}/health")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body, serde_json::json!({ "status": "ok" }));

    // Test that health endpoint is NOT accessible at versioned path
    let resp = reqwest::get(format!("http://{addr}/{API_VERSION}/health")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    server.abort();
}
