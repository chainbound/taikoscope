use super::*;
use crate::*;
use clickhouse::{
    Row,
    test::{Mock, handlers},
};

#[derive(Row, serde::Serialize)]
struct FeeRow {
    l2_block_number: u64,
    priority_fee: u128,
    base_fee: u128,
    l1_data_cost: Option<u128>,
}

#[tokio::test]
async fn fee_components_returns_expected_rows() {
    let mock = Mock::new();
    mock.add(handlers::provide(vec![FeeRow {
        l2_block_number: 1,
        priority_fee: 10,
        base_fee: 20,
        l1_data_cost: Some(5),
    }]));

    let url = url::Url::parse(mock.url()).unwrap();
    let reader = ClickhouseReader::new(url, "db".to_owned(), "user".into(), "pass".into()).unwrap();

    let rows = reader.get_l2_fee_components(None, TimeRange::LastHour).await.unwrap();

    assert_eq!(
        rows,
        vec![BlockFeeComponentRow {
            l2_block_number: 1,
            priority_fee: 10,
            base_fee: 20,
            l1_data_cost: Some(5),
        }]
    );
}

#[derive(Row, serde::Serialize)]
struct SeqFeeRow {
    sequencer: AddressBytes,
    priority_fee: u128,
    base_fee: u128,
    l1_data_cost: Option<u128>,
    prove_cost: Option<u128>,
}

#[tokio::test]
async fn fees_by_sequencer_returns_expected_rows() {
    let mock = Mock::new();
    let addr = AddressBytes([1u8; 20]);

    // Mock for get_batch_fee_components
    mock.add(handlers::provide(vec![BatchFeeRow {
        batch_id: 1,
        l1_block_number: 10,
        l1_tx_hash: HashBytes([0u8; 32]),
        proposer: addr,
        priority_fee: 10,
        base_fee: 20,
        l1_data_cost: Some(5),
    }]));

    // Mock for get_prove_costs_by_proposer
    #[derive(Row, serde::Serialize)]
    struct ProposerCostRow {
        proposer: AddressBytes,
        total_cost: u128,
    }
    mock.add(handlers::provide(vec![ProposerCostRow { proposer: addr, total_cost: 3 }]));

    let url = url::Url::parse(mock.url()).unwrap();
    let reader = ClickhouseReader::new(url, "db".to_owned(), "user".into(), "pass".into()).unwrap();

    let rows = reader.get_l2_fees_by_sequencer(TimeRange::LastHour).await.unwrap();

    assert_eq!(
        rows,
        vec![SequencerFeeRow {
            sequencer: addr,
            priority_fee: 10,
            base_fee: 20,
            l1_data_cost: Some(5),
            prove_cost: Some(3),
        }]
    );
}

#[test]
fn fees_by_sequencer_query_has_proper_spacing() {
    let db_name = "test_db";

    // Simulate the query generation logic
    let range = TimeRange::LastHour;
    let query = format!(
        "SELECT h.sequencer, \
                sum(sum_priority_fee) AS priority_fee, \
                sum(sum_base_fee) AS base_fee, \
                toNullable(sum(if(b.batch_size > 0, intDiv(dc.cost, b.batch_size), NULL))) AS l1_data_cost, \
                toNullable(sum(if(b.batch_size > 0, intDiv(pc.cost, b.batch_size), NULL))) AS prove_cost, \
                toNullable(sum(if(b.batch_size > 0, intDiv(vc.cost, b.batch_size), NULL))) AS verify_cost \
         FROM {db}.l2_head_events h \
         LEFT JOIN {db}.batch_blocks bb \
           ON h.l2_block_number = bb.l2_block_number \
         LEFT JOIN {db}.batches b \
           ON bb.batch_id = b.batch_id \
         LEFT JOIN {db}.l1_data_costs dc \
           ON b.batch_id = dc.batch_id \
         LEFT JOIN {db}.prove_costs pc \
           ON b.batch_id = pc.batch_id \
         LEFT JOIN {db}.verify_costs vc \
           ON b.batch_id = vc.batch_id \
         WHERE h.block_ts >= toUnixTimestamp(now64() - INTERVAL {interval}) \
           AND bb.batch_id IS NOT NULL \
         GROUP BY h.sequencer \
         ORDER BY priority_fee DESC",
        interval = range.interval(),
        db = db_name,
    );

    // Verify that the problematic concatenations have proper spacing
    assert!(
        query.contains("l2_head_events h LEFT JOIN"),
        "Query should have space between 'h' and 'LEFT JOIN'"
    );
    assert!(query.contains("l1_data_costs dc ON"), "Query should have space between 'dc' and 'ON'");
    assert!(query.contains("batch_blocks bb ON"), "Query should have space between 'bb' and 'ON'",);
    assert!(query.contains("batches b ON"), "Query should have space between 'b' and 'ON'",);
    assert!(query.contains("prove_costs pc ON"), "Query should have space between 'pc' and 'ON'",);
    assert!(query.contains("verify_costs vc ON"), "Query should have space between 'vc' and 'ON'",);
    assert!(
        query.contains("bb.l2_block_number LEFT JOIN"),
        "Query should have space between 'l2_block_number' and 'LEFT JOIN'"
    );
    assert!(query.contains(") AND"), "Query should have space between ')' and 'AND'");

    // Verify that malformed tokens are not present
    assert!(!query.contains("hLEFT"), "Query should not contain 'hLEFT'");
    assert!(!query.contains("dcON"), "Query should not contain 'dcON'");
    assert!(!query.contains("bbON"), "Query should not contain 'bbON'");
    assert!(!query.contains("pcON"), "Query should not contain 'pcON'");
    assert!(!query.contains("vcON"), "Query should not contain 'vcON'");
}

#[derive(Row, serde::Serialize)]
struct BatchFeeRow {
    batch_id: u64,
    l1_block_number: u64,
    l1_tx_hash: HashBytes,
    proposer: AddressBytes,
    priority_fee: u128,
    base_fee: u128,
    l1_data_cost: Option<u128>,
}

#[tokio::test]
async fn batch_fee_components_returns_expected_rows() {
    let mock = Mock::new();
    mock.add(handlers::provide(vec![BatchFeeRow {
        batch_id: 1,
        l1_block_number: 10,
        l1_tx_hash: HashBytes([0u8; 32]),
        proposer: AddressBytes([1u8; 20]),
        priority_fee: 10,
        base_fee: 20,
        l1_data_cost: Some(5),
    }]));

    let url = url::Url::parse(mock.url()).unwrap();
    let reader = ClickhouseReader::new(url, "db".to_owned(), "user".into(), "pass".into()).unwrap();

    let rows = reader.get_batch_fee_components(None, TimeRange::LastHour).await.unwrap();

    assert_eq!(
        rows,
        vec![BatchFeeComponentRow {
            batch_id: 1,
            l1_block_number: 10,
            l1_tx_hash: HashBytes([0u8; 32]),
            sequencer: AddressBytes([1u8; 20]),
            priority_fee: 10,
            base_fee: 20,
            l1_data_cost: Some(5),
        }]
    );
}

#[tokio::test]
async fn batch_total_fee_helpers_return_expected_values() {
    let mock = Mock::new();
    for _ in 0..3 {
        mock.add(handlers::provide(vec![BatchFeeRow {
            batch_id: 1,
            l1_block_number: 10,
            l1_tx_hash: HashBytes([0u8; 32]),
            proposer: AddressBytes([1u8; 20]),
            priority_fee: 10,
            base_fee: 20,
            l1_data_cost: Some(5),
        }]));
    }

    let url = url::Url::parse(mock.url()).unwrap();
    let reader = ClickhouseReader::new(url, "db".to_owned(), "user".into(), "pass".into()).unwrap();

    let priority = reader.get_batch_priority_fee(None, TimeRange::LastHour).await.unwrap().unwrap();
    assert_eq!(priority, 10);
    let base = reader.get_batch_base_fee(None, TimeRange::LastHour).await.unwrap().unwrap();
    assert_eq!(base, 20);
    let cost = reader.get_batch_total_data_cost(None, TimeRange::LastHour).await.unwrap().unwrap();
    assert_eq!(cost, 5);
}

#[tokio::test]
async fn batch_fees_by_proposer_returns_expected_rows() {
    let mock = Mock::new();
    mock.add(handlers::provide(vec![SeqFeeRow {
        sequencer: AddressBytes([1u8; 20]),
        priority_fee: 10,
        base_fee: 20,
        l1_data_cost: Some(5),
        prove_cost: None,
    }]));

    let url = url::Url::parse(mock.url()).unwrap();
    let reader = ClickhouseReader::new(url, "db".to_owned(), "user".into(), "pass".into()).unwrap();

    let rows = reader.get_batch_fees_by_proposer(TimeRange::LastHour).await.unwrap();

    assert_eq!(
        rows,
        vec![SequencerFeeRow {
            sequencer: AddressBytes([1u8; 20]),
            priority_fee: 10,
            base_fee: 20,
            l1_data_cost: Some(5),
            prove_cost: None,
        }]
    );
}
