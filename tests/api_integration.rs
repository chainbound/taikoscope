use std::{net::SocketAddr, time::Duration};

use clickhouse::{
    Row,
    test::{Mock, handlers},
};
use mockito;
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
use primitives::WEI_PER_GWEI;
use server::{API_VERSION, router};
use tokio::net::TcpListener;

#[derive(Serialize, Row)]
struct MaxRow {
    block_ts: u64,
}

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
async fn batch_fee_components_filters_unverified() {
    let mock = Mock::new();
    mock.add(handlers::provide::<VerifiedBatchRow>(vec![VerifiedBatchRow {
        l1_block_number: 10,
        batch_id: 1,
        block_hash: HashBytes([0u8; 32]),
    }]));
    mock.add(handlers::provide(vec![
        BatchFeeRow {
            batch_id: 1,
            priority_fee: 10 * WEI_PER_GWEI,
            base_fee: 20 * WEI_PER_GWEI,
            l1_data_cost: Some(5 * WEI_PER_GWEI),
        },
        BatchFeeRow {
            batch_id: 2,
            priority_fee: 1 * WEI_PER_GWEI,
            base_fee: 2 * WEI_PER_GWEI,
            l1_data_cost: Some(1 * WEI_PER_GWEI),
        },
    ]));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let resp = reqwest::get(format!(
        "http://{addr}/{API_VERSION}/batch-fee-components?created[gte]=0&created[lte]=3600000"
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "batches": [
                {
                    "batch_id": 1,
                    "priority_fee": 10,
                    "base_fee": 20,
                    "l1_data_cost": 5,
                    "amortized_prove_cost": null
                }
            ]
        })
    );

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

#[derive(Serialize, Row)]
struct BlockNumRow {
    l1_block_number: u64,
}

#[derive(Serialize, Row)]
struct BlockTimeRow {
    minute: u64,
    l1_block_number: u64,
}

#[derive(Serialize, Row)]
struct FeeRow {
    l2_block_number: u64,
    priority_fee: u128,
    base_fee: u128,
    l1_data_cost: Option<u128>,
}

#[derive(Serialize, Row)]
struct BlobRow {
    l1_block_number: u64,
    batch_id: u64,
    blob_count: u8,
}

#[derive(Serialize, Row)]
struct BatchFeeRow {
    batch_id: u64,
    priority_fee: u128,
    base_fee: u128,
    l1_data_cost: Option<u128>,
}

#[derive(Serialize, Row)]
struct TotalRow {
    total: u128,
}

#[derive(Serialize, Row)]
struct AggregatedCostRow {
    proposer: AddressBytes,
    total_cost: u128,
}

#[derive(Serialize, Row)]
struct VerifiedBatchRow {
    l1_block_number: u64,
    batch_id: u64,
    block_hash: HashBytes,
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

#[tokio::test]
async fn l1_head_block_integration() {
    let mock = Mock::new();
    mock.add(handlers::provide(vec![BlockNumRow { l1_block_number: 3 }]));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let resp = reqwest::get(format!("http://{addr}/{API_VERSION}/l1-head-block")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body, serde_json::json!({ "l1_head_block": 3 }));

    server.abort();
}

#[tokio::test]
async fn l1_block_times_success_and_invalid() {
    let mock = Mock::new();
    mock.add(handlers::provide(vec![BlockTimeRow { minute: 1, l1_block_number: 2 }]));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let resp = reqwest::get(format!(
        "http://{addr}/{API_VERSION}/l1-block-times?created[gte]=0&created[lte]=3600000"
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body, serde_json::json!({ "blocks": [ { "minute": 1, "l1_block_number": 2 } ] }));

    let resp = reqwest::get(format!(
        "http://{addr}/{API_VERSION}/l1-block-times?created[gte]=10&created[lte]=5"
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    server.abort();
}

#[tokio::test]
async fn l2_fee_components_aggregated_integration() {
    let mock = Mock::new();
    mock.add(handlers::provide(vec![
        FeeRow {
            l2_block_number: 0,
            priority_fee: 10 * WEI_PER_GWEI,
            base_fee: 20 * WEI_PER_GWEI,
            l1_data_cost: Some(5 * WEI_PER_GWEI),
        },
        FeeRow {
            l2_block_number: 1,
            priority_fee: 1 * WEI_PER_GWEI,
            base_fee: 2 * WEI_PER_GWEI,
            l1_data_cost: None,
        },
    ]));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let resp = reqwest::get(
        format!("http://{addr}/{API_VERSION}/l2-fee-components/aggregated?created[gte]=0&created[lte]=86400000"),
    )
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "blocks": [
                { "l2_block_number": 0, "priority_fee": 11, "base_fee": 22, "l1_data_cost": 5 }
            ]
        })
    );

    let resp = reqwest::get(
        format!("http://{addr}/{API_VERSION}/l2-fee-components/aggregated?created[gte]=0&created[lte]=3600000&address=zzz"),
    )
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    server.abort();
}

#[tokio::test]
async fn blobs_per_batch_desc_order() {
    let mock = Mock::new();
    mock.add(handlers::provide(vec![
        BlobRow { l1_block_number: 5, batch_id: 2, blob_count: 3 },
        BlobRow { l1_block_number: 4, batch_id: 1, blob_count: 1 },
    ]));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let resp = reqwest::get(format!("http://{addr}/{API_VERSION}/blobs-per-batch?limit=10"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "batches": [
                { "l1_block_number": 5, "batch_id": 2, "blob_count": 3 },
                { "l1_block_number": 4, "batch_id": 1, "blob_count": 1 }
            ]
        })
    );

    server.abort();
}

#[tokio::test]
async fn block_profits_integration() {
    let mock = Mock::new();
    mock.add(handlers::provide(vec![
        FeeRow { l2_block_number: 1, priority_fee: 5, base_fee: 10, l1_data_cost: Some(3) },
        FeeRow { l2_block_number: 2, priority_fee: 2, base_fee: 2, l1_data_cost: Some(10) },
    ]));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let resp = reqwest::get(
        format!("http://{addr}/{API_VERSION}/block-profits?created[gte]=0&created[lte]=3600000&limit=1&order=desc"),
    )
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body, serde_json::json!({ "blocks": [ { "block": 1, "profit": 12 } ] }));

    let resp = reqwest::get(
        format!("http://{addr}/{API_VERSION}/block-profits?created[gte]=0&created[lte]=3600000&limit=1&order=asc"),
    )
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body, serde_json::json!({ "blocks": [ { "block": 2, "profit": -6 } ] }));

    server.abort();
}

#[tokio::test]
async fn batch_fee_components_integration() {
    let mock = Mock::new();
    mock.add(handlers::provide::<VerifiedBatchRow>(vec![VerifiedBatchRow {
        l1_block_number: 10,
        batch_id: 1,
        block_hash: HashBytes([0u8; 32]),
    }]));
    mock.add(handlers::provide(vec![BatchFeeRow {
        batch_id: 1,
        priority_fee: 10 * WEI_PER_GWEI,
        base_fee: 20 * WEI_PER_GWEI,
        l1_data_cost: Some(5 * WEI_PER_GWEI),
    }]));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let resp = reqwest::get(format!(
        "http://{addr}/{API_VERSION}/batch-fee-components?created[gte]=0&created[lte]=3600000"
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "batches": [
                {
                    "batch_id": 1,
                    "priority_fee": 10,
                    "base_fee": 20,
                    "l1_data_cost": 5,
                    "amortized_prove_cost": null
                }
            ]
        })
    );

    server.abort();
}

#[tokio::test]
async fn verify_times_integration() {
    let mock = Mock::new();
    mock.add(handlers::provide(vec![clickhouse_lib::BatchVerifyTimeRow {
        l1_block_number: 2,
        batch_id: 1,
        seconds_to_verify: 456,
    }]));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let resp = reqwest::get(format!(
        "http://{addr}/{API_VERSION}/verify-times?created[gte]=0&created[lte]=3600000"
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "batches": [ { "l1_block_number": 2, "batch_id": 1, "seconds_to_verify": 456 } ]
        })
    );

    server.abort();
}

#[tokio::test]
async fn prove_cost_integration() {
    let mock = Mock::new();
    mock.add(handlers::provide(vec![clickhouse_lib::ProveCostRow {
        l1_block_number: 3,
        batch_id: 1,
        cost: 123 * WEI_PER_GWEI,
    }]));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let resp = reqwest::get(format!(
        "http://{addr}/{API_VERSION}/prove-cost?created[gte]=0&created[lte]=3600000"
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "batches": [ { "l1_block_number": 3, "batch_id": 1, "cost": 123 } ]
        })
    );

    server.abort();
}



#[tokio::test]
async fn l2_fees_integration() {
    let mock = Mock::new();
    mock.add(handlers::provide(vec![TotalRow { total: 600 * WEI_PER_GWEI }]));
    mock.add(handlers::provide(vec![TotalRow { total: 400 * WEI_PER_GWEI }]));
    mock.add(handlers::provide(vec![TotalRow { total: 10 * WEI_PER_GWEI }]));
    mock.add(handlers::provide::<clickhouse_lib::SequencerFeeRow>(vec![
        clickhouse_lib::SequencerFeeRow {
            sequencer: AddressBytes([1u8; 20]),
            priority_fee: 600 * WEI_PER_GWEI,
            base_fee: 400 * WEI_PER_GWEI,
            l1_data_cost: Some(10 * WEI_PER_GWEI),
            prove_cost: Some(5 * WEI_PER_GWEI),
        },
    ]));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let resp = reqwest::get(format!(
        "http://{addr}/{API_VERSION}/l2-fees?created[gte]=0&created[lte]=3600000"
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "priority_fee": 600,
            "base_fee": 400,
            "l1_data_cost": 10,
            "prove_cost": 5,
            "sequencers": [
                {
                    "address": format!("0x{}", hex::encode([1u8; 20])),
                    "priority_fee": 600,
                    "base_fee": 400,
                    "l1_data_cost": 10,
                    "prove_cost": 5
                }
            ]
        })
    );

    server.abort();
}

#[tokio::test]
async fn batch_fees_integration() {
    let mock = Mock::new();
    mock.add(handlers::provide(vec![TotalRow { total: 10 * WEI_PER_GWEI }]));
    mock.add(handlers::provide(vec![TotalRow { total: 20 * WEI_PER_GWEI }]));
    mock.add(handlers::provide(vec![TotalRow { total: 5 * WEI_PER_GWEI }]));
    mock.add(handlers::provide::<clickhouse_lib::SequencerFeeRow>(vec![
        clickhouse_lib::SequencerFeeRow {
            sequencer: AddressBytes([2u8; 20]),
            priority_fee: 10 * WEI_PER_GWEI,
            base_fee: 20 * WEI_PER_GWEI,
            l1_data_cost: Some(5 * WEI_PER_GWEI),
            prove_cost: Some(1 * WEI_PER_GWEI),
        },
    ]));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let resp = reqwest::get(format!(
        "http://{addr}/{API_VERSION}/batch-fees?created[gte]=0&created[lte]=3600000"
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "priority_fee": 10,
            "base_fee": 20,
            "l1_data_cost": 5,
            "prove_cost": null,
            "sequencers": [
                {
                    "address": format!("0x{}", hex::encode([2u8; 20])),
                    "priority_fee": 10,
                    "base_fee": 20,
                    "l1_data_cost": 5,
                    "prove_cost": 1
                }
            ]
        })
    );

    server.abort();
}

#[tokio::test]
async fn prove_costs_integration() {
    let mock = Mock::new();
    mock.add(handlers::provide(vec![AggregatedCostRow {
        proposer: AddressBytes([3u8; 20]),
        total_cost: 123 * WEI_PER_GWEI,
    }]));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let resp = reqwest::get(format!(
        "http://{addr}/{API_VERSION}/prove-costs?created[gte]=0&created[lte]=3600000"
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "proposers": [
                {
                    "address": format!("0x{}", hex::encode([3u8; 20])),
                    "cost": 123
                }
            ]
        })
    );

    server.abort();
}



#[tokio::test]
async fn l1_data_cost_paginated() {
    let mock = Mock::new();
    mock.add(handlers::provide(vec![clickhouse_lib::L1DataCostRow {
        l1_block_number: 1,
        cost: 789 * WEI_PER_GWEI,
    }]));

    let url = Url::parse(mock.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let resp = reqwest::get(format!(
        "http://{addr}/{API_VERSION}/l1-data-cost?starting_after=0&ending_before=2&limit=1"
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "blocks": [ { "l1_block_number": 1, "cost": 789 } ]
        })
    );

    server.abort();
}

#[tokio::test]
async fn eth_price_cached() {
    let ck = Mock::new();
    ck.add(handlers::provide(vec![MaxRow { block_ts: 1 }]));

    let url = Url::parse(ck.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let mut price_server = mockito::Server::new_async().await;
    let mock = price_server
        .mock("GET", "/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("{\"ethereum\":{\"usd\":123.45}}")
        .expect(1)
        .create_async()
        .await;

    std::env::set_var("ETH_PRICE_URL", price_server.url());

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let url = format!("http://{addr}/{API_VERSION}/eth-price");
    let resp1 = reqwest::get(&url).await.unwrap();
    assert_eq!(resp1.status(), StatusCode::OK);
    let body: serde_json::Value = resp1.json().await.unwrap();
    assert_eq!(body, serde_json::json!({ "price": 123.45 }));

    let resp2 = reqwest::get(&url).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);

    mock.assert_async().await;
    std::env::remove_var("ETH_PRICE_URL");
    server.abort();
}

#[tokio::test]
async fn eth_price_handles_429() {
    let ck = Mock::new();
    ck.add(handlers::provide(vec![MaxRow { block_ts: 1 }]));

    let url = Url::parse(ck.url()).unwrap();
    let client =
        ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();

    let mut price_server = mockito::Server::new_async().await;
    let mock = price_server.mock("GET", "/").with_status(429).expect(1).create_async().await;

    std::env::set_var("ETH_PRICE_URL", price_server.url());

    let (addr, server) = spawn_server(client).await;
    wait_for_server(addr).await;

    let url = format!("http://{addr}/{API_VERSION}/eth-price");
    let resp = reqwest::get(url).await.unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["type"], "price-error");

    mock.assert_async().await;
    std::env::remove_var("ETH_PRICE_URL");
    server.abort();
}
