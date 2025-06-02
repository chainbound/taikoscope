use std::{net::SocketAddr, time::Duration};

use clickhouse::{
    Row,
    test::{Mock, handlers},
};
use reqwest::StatusCode;
use serde::Serialize;
use tokio::time::sleep;
use url::Url;

use api::ApiState;
use server::{router, API_VERSION};
use axum::serve;
use tokio::net::TcpListener;
use clickhouse_lib::ClickhouseReader;

#[derive(Serialize, Row)]
struct MaxRow {
    block_ts: u64,
}

async fn spawn_server(client: ClickhouseReader) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let state = ApiState::new(client);
    let app = router(state, vec![]);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(serve(listener, app));
    (addr, handle)
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

    sleep(Duration::from_millis(100)).await;

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

    sleep(Duration::from_millis(100)).await;

    let resp = reqwest::get(format!("http://{addr}/l1-head")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    let expected = chrono::Utc.timestamp_opt(ts as i64, 0).single().unwrap().to_rfc3339();
    assert_eq!(body, serde_json::json!({ "last_l1_head_time": expected }));

    server.abort();
}

#[derive(Serialize, Row)]
struct MaxNum {
    number: u64,
}

#[tokio::test]
async fn l2_head_block_integration() {
    let mock = Mock::new();
    mock.add(handlers::provide(vec![MaxNum { number: 5 }]));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;

    sleep(Duration::from_millis(100)).await;

    let resp = reqwest::get(format!("http://{addr}/l2-head-block")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body, serde_json::json!({ "l2_head_block": 5 }));

    server.abort();
}
