use std::{net::SocketAddr, time::Duration};

use axum::{extract::connect_info::IntoMakeServiceWithConnectInfo, serve};
use chrono::Utc;
use clickhouse::{
    Row,
    test::{Mock, handlers},
};
use reqwest::StatusCode;
use tokio::{
    net::{TcpListener, TcpStream},
    time::{Instant, sleep},
};
use url::Url;

use api::{ApiState, DEFAULT_MAX_REQUESTS, DEFAULT_RATE_PERIOD};
use clickhouse_lib::ClickhouseReader;
use server::{API_VERSION, router};

async fn spawn_server(client: ClickhouseReader) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let state = ApiState::new(client, DEFAULT_MAX_REQUESTS, DEFAULT_RATE_PERIOD);
    let allowed = config::DEFAULT_ALLOWED_ORIGINS.split(',').map(|s| s.to_owned()).collect();
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
async fn l2_reorgs_paginated_builds_query() {
    let mock = Mock::new();
    let ctl = mock.add(handlers::record_ddl());
    let url = Url::parse(mock.url()).unwrap();
    let reader = ClickhouseReader::new(url, "db".to_owned(), "user".into(), "pass".into()).unwrap();

    let since = Utc.timestamp_opt(0, 0).single().unwrap();
    let _ = reader.get_l2_reorgs_paginated(since, 5, Some(100), Some(50)).await;
    let query = ctl.query().await;
    assert!(query.contains("l2_block_number < 100"));
    assert!(query.contains("l2_block_number > 50"));
    assert!(query.contains("LIMIT 5"));
    assert!(query.contains("ORDER BY l2_block_number DESC"));
}

#[tokio::test]
async fn batch_posting_times_paginated_builds_query() {
    let mock = Mock::new();
    let ctl = mock.add(handlers::record_ddl());
    let url = Url::parse(mock.url()).unwrap();
    let reader = ClickhouseReader::new(url, "db".to_owned(), "user".into(), "pass".into()).unwrap();

    let since = Utc.timestamp_opt(0, 0).single().unwrap();
    let _ = reader.get_batch_posting_times_paginated(since, 10, Some(5), Some(20)).await;
    let query = ctl.query().await;
    assert!(query.contains("batch_id > 5"));
    assert!(query.contains("batch_id < 20"));
    assert!(query.contains("LIMIT 10"));
    assert!(query.contains("ORDER BY batch_id ASC"));
}

#[tokio::test]
async fn reorgs_endpoint_returns_items_with_pagination() {
    #[derive(Serialize, Row)]
    struct RawRow {
        l2_block_number: u64,
        depth: u16,
        ts: u64,
    }

    let mock = Mock::new();
    mock.add(handlers::provide(vec![
        RawRow { l2_block_number: 9, depth: 1, ts: 1000 },
        RawRow { l2_block_number: 8, depth: 2, ts: 2000 },
    ]));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let resp = reqwest::get(format!(
        "http://{addr}/{API_VERSION}/reorgs?starting_after=7&ending_before=10&limit=2"
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    let expected = serde_json::json!({
        "events": [
            {
                "l2_block_number": 9,
                "depth": 1,
                "inserted_at": Utc.timestamp_millis_opt(1000).single().unwrap().to_rfc3339()
            },
            {
                "l2_block_number": 8,
                "depth": 2,
                "inserted_at": Utc.timestamp_millis_opt(2000).single().unwrap().to_rfc3339()
            }
        ]
    });
    assert_eq!(body, expected);

    server.abort();
}

#[tokio::test]
async fn batch_posting_times_endpoint_returns_items_with_pagination() {
    #[derive(Serialize, Row)]
    struct RawRow {
        batch_id: u64,
        ts: u64,
        ms_since_prev_batch: Option<u64>,
    }

    let mock = Mock::new();
    mock.add(handlers::provide(vec![RawRow {
        batch_id: 1,
        ts: 1000,
        ms_since_prev_batch: Some(500),
    }]));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let resp = reqwest::get(format!(
        "http://{addr}/{API_VERSION}/batch-posting-times?starting_after=0&ending_before=2&limit=1"
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    let expected = serde_json::json!({
        "batches": [
            {
                "batch_id": 1,
                "inserted_at": Utc.timestamp_millis_opt(1000).single().unwrap().to_rfc3339(),
                "ms_since_prev_batch": 500
            }
        ]
    });
    assert_eq!(body, expected);

    server.abort();
}
