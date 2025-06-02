use std::{net::SocketAddr, time::Duration};

use clickhouse::{
    Row,
    test::{Mock, handlers},
};
use reqwest::StatusCode;
use serde::Serialize;
use tokio::time::sleep;
use url::Url;

use server::run;
use clickhouse_lib::ClickhouseReader;

#[derive(Serialize, Row)]
struct MaxRow {
    block_ts: u64,
}

#[tokio::test]
async fn l2_head_integration() {
    let mock = Mock::new();
    let ts = 42u64;
    mock.add(handlers::provide(vec![MaxRow { block_ts: ts }]));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let addr: SocketAddr = "127.0.0.1:3001".parse().unwrap();
    let server = tokio::spawn(run(
        addr,
        client,
        vec![],
        1000,
        Duration::from_secs(60),
    ));

    sleep(Duration::from_millis(100)).await;

    let resp = reqwest::get(format!("http://{addr}/l2-head")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    let expected = chrono::Utc.timestamp_opt(ts as i64, 0).single().unwrap().to_rfc3339();
    assert_eq!(body, serde_json::json!({ "last_l2_head_time": expected }));

    server.abort();
}
