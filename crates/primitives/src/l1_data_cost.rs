use alloy_consensus::transaction::Transaction;
use alloy_network_primitives::ReceiptResponse;
use alloy_primitives::Address;

/// Returns the total fee paid for blobs in the given receipt.
/// If the receipt does not contain blob data, `0` is returned.
pub fn calculate_blob_fee_from_receipt<R: ReceiptResponse>(receipt: &R) -> u128 {
    match (receipt.blob_gas_used(), receipt.blob_gas_price()) {
        (Some(gas_used), Some(price)) => (gas_used as u128).saturating_mul(price),
        _ => 0,
    }
}

/// Calculate the total cost for a single receipt (execution + blob fees).
/// This function computes the cost for any transaction receipt without filtering.
pub fn cost_from_receipt<R: ReceiptResponse>(receipt: &R) -> u128 {
    // Calculate execution gas cost (gas_used * effective_gas_price)
    let exec_cost = (receipt.gas_used() as u128).saturating_mul(receipt.effective_gas_price());

    // Calculate blob gas cost if present
    let blob_cost = calculate_blob_fee_from_receipt(receipt);

    // Return total cost
    exec_cost.saturating_add(blob_cost)
}

/// Compute the total L1 data posting cost for the given transactions and receipts.
/// Only transactions sent to the provided inbox address are considered.
pub fn compute_l1_data_posting_cost<T, R>(txs: &[T], receipts: &[R], inbox: Address) -> u128
where
    T: Transaction,
    R: ReceiptResponse,
{
    let mut total = 0u128;
    for (tx, receipt) in txs.iter().zip(receipts) {
        // Only consider transactions sent to the inbox address
        if tx.to() == Some(inbox) {
            // Calculate execution gas cost (gas_used * effective_gas_price)
            let exec_cost =
                (receipt.gas_used() as u128).saturating_mul(receipt.effective_gas_price());

            // Calculate blob gas cost if present
            let blob_cost = calculate_blob_fee_from_receipt(receipt);

            // Add both costs to total
            total = total.saturating_add(exec_cost.saturating_add(blob_cost));
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Address, B256, BlockHash, TxHash, address};

    #[derive(Debug, Clone, Copy)]
    struct TestReceipt {
        gas: u64,
        price: u128,
        blob_gas: Option<u64>,
        blob_price: Option<u128>,
    }

    impl ReceiptResponse for TestReceipt {
        fn contract_address(&self) -> Option<Address> {
            None
        }
        fn status(&self) -> bool {
            true
        }
        fn block_hash(&self) -> Option<BlockHash> {
            None
        }
        fn block_number(&self) -> Option<u64> {
            None
        }
        fn transaction_hash(&self) -> TxHash {
            TxHash::ZERO
        }
        fn transaction_index(&self) -> Option<u64> {
            None
        }
        fn gas_used(&self) -> u64 {
            self.gas
        }
        fn effective_gas_price(&self) -> u128 {
            self.price
        }
        fn blob_gas_used(&self) -> Option<u64> {
            self.blob_gas
        }
        fn blob_gas_price(&self) -> Option<u128> {
            self.blob_price
        }
        fn from(&self) -> Address {
            address!("0x0000000000000000000000000000000000000000")
        }
        fn to(&self) -> Option<Address> {
            None
        }
        fn cumulative_gas_used(&self) -> u64 {
            self.gas
        }
        fn state_root(&self) -> Option<B256> {
            None
        }
    }

    #[test]
    fn sums_matching_transactions() {
        let inbox = address!("0x000000000000000000000000000000000000dead");
        let tx = alloy_consensus::TxEip4844 {
            to: inbox,
            blob_versioned_hashes: vec![B256::ZERO],
            ..Default::default()
        };
        let receipt = TestReceipt { gas: 10, price: 2, blob_gas: Some(3), blob_price: Some(4) };
        let cost = compute_l1_data_posting_cost(&[tx], &[receipt], inbox);
        // Should count both execution cost (10 * 2) and blob cost (3 * 4) = 20 + 12 = 32
        assert_eq!(cost, 32);
    }

    #[test]
    fn ignores_non_matching_transactions() {
        let inbox = address!("0x000000000000000000000000000000000000dead");
        let wrong_address = address!("0x000000000000000000000000000000000000beef");
        let tx = alloy_consensus::TxEip4844 {
            to: wrong_address, // Wrong address, should be ignored
            blob_versioned_hashes: vec![],
            ..Default::default()
        };
        let receipt = TestReceipt { gas: 10, price: 2, blob_gas: Some(3), blob_price: Some(4) };
        let cost = compute_l1_data_posting_cost(&[tx], &[receipt], inbox);
        assert_eq!(cost, 0);
    }

    #[test]
    fn counts_transactions_without_blobs_to_inbox() {
        let inbox = address!("0x000000000000000000000000000000000000dead");
        let tx = alloy_consensus::TxEip4844 {
            to: inbox,
            blob_versioned_hashes: vec![], // No blobs, but should still count
            ..Default::default()
        };
        let receipt = TestReceipt { gas: 10, price: 2, blob_gas: None, blob_price: None };
        let cost = compute_l1_data_posting_cost(&[tx], &[receipt], inbox);
        // Should count execution cost: 10 * 2 = 20
        assert_eq!(cost, 20);
    }

    #[test]
    fn counts_transactions_with_execution_and_blob_costs() {
        let inbox = address!("0x000000000000000000000000000000000000dead");
        let tx = alloy_consensus::TxEip4844 {
            to: inbox,
            blob_versioned_hashes: vec![B256::ZERO],
            ..Default::default()
        };
        let receipt = TestReceipt { gas: 10, price: 2, blob_gas: Some(3), blob_price: Some(4) };
        let cost = compute_l1_data_posting_cost(&[tx], &[receipt], inbox);
        // Should count both execution cost (10 * 2) and blob cost (3 * 4) = 20 + 12 = 32
        assert_eq!(cost, 32);
    }

    #[test]
    fn blob_fee_calculation() {
        let receipt = TestReceipt { blob_gas: Some(100), blob_price: Some(10), gas: 0, price: 0 };
        assert_eq!(calculate_blob_fee_from_receipt(&receipt), 1000);
    }

    #[test]
    fn blob_fee_no_fee() {
        let receipt = TestReceipt { blob_gas: None, blob_price: Some(10), gas: 0, price: 0 };
        assert_eq!(calculate_blob_fee_from_receipt(&receipt), 0);

        let receipt = TestReceipt { blob_gas: Some(100), blob_price: None, gas: 0, price: 0 };
        assert_eq!(calculate_blob_fee_from_receipt(&receipt), 0);

        let receipt = TestReceipt { blob_gas: None, blob_price: None, gas: 0, price: 0 };
        assert_eq!(calculate_blob_fee_from_receipt(&receipt), 0);
    }

    #[test]
    fn blob_fee_saturating_mul() {
        let receipt =
            TestReceipt { blob_gas: Some(u64::MAX), blob_price: Some(u128::MAX), gas: 0, price: 0 };
        assert_eq!(calculate_blob_fee_from_receipt(&receipt), u128::MAX);
    }

    #[test]
    fn calculates_cost_components_like_batch_receipt() {
        let inbox = address!("0x000000000000000000000000000000000000dead");
        let tx = alloy_consensus::TxEip4844 {
            to: inbox,
            blob_versioned_hashes: vec![B256::ZERO],
            ..Default::default()
        };
        // Simulating a receipt with gas_used=1000, effective_gas_price=50, blob_gas_used=200,
        // blob_gas_price=10
        let receipt =
            TestReceipt { gas: 1000, price: 50, blob_gas: Some(200), blob_price: Some(10) };
        let cost = compute_l1_data_posting_cost(&[tx], &[receipt], inbox);

        // Components:
        // exec_cost = gas_used * effective_gas_price = 1000 * 50 = 50000
        // blob_cost = blob_gas_used * blob_gas_price = 200 * 10 = 2000
        // total_cost = exec_cost + blob_cost = 50000 + 2000 = 52000
        assert_eq!(cost, 52000);
    }

    #[test]
    fn calculates_multiple_transactions_to_inbox() {
        let inbox = address!("0x000000000000000000000000000000000000dead");
        let other_address = address!("0x000000000000000000000000000000000000beef");

        let tx1 = alloy_consensus::TxEip4844 {
            to: inbox, // Should be counted
            blob_versioned_hashes: vec![],
            ..Default::default()
        };
        let tx2 = alloy_consensus::TxEip4844 {
            to: other_address, // Should be ignored
            blob_versioned_hashes: vec![B256::ZERO],
            ..Default::default()
        };
        let tx3 = alloy_consensus::TxEip4844 {
            to: inbox, // Should be counted
            blob_versioned_hashes: vec![B256::ZERO],
            ..Default::default()
        };

        let receipt1 = TestReceipt { gas: 100, price: 2, blob_gas: None, blob_price: None };
        let receipt2 = TestReceipt { gas: 500, price: 3, blob_gas: Some(50), blob_price: Some(4) };
        let receipt3 = TestReceipt { gas: 200, price: 5, blob_gas: Some(30), blob_price: Some(6) };

        let cost =
            compute_l1_data_posting_cost(&[tx1, tx2, tx3], &[receipt1, receipt2, receipt3], inbox);

        // Only tx1 and tx3 should be counted (sent to inbox):
        // tx1: exec_cost = 100 * 2 = 200, blob_cost = 0, total = 200
        // tx3: exec_cost = 200 * 5 = 1000, blob_cost = 30 * 6 = 180, total = 1180
        // Total = 200 + 1180 = 1380
        assert_eq!(cost, 1380);
    }

    #[test]
    fn cost_from_receipt_with_both_fees() {
        let receipt =
            TestReceipt { gas: 1000, price: 50, blob_gas: Some(200), blob_price: Some(10) };
        let cost = cost_from_receipt(&receipt);
        // exec_cost = 1000 * 50 = 50000
        // blob_cost = 200 * 10 = 2000
        // total = 50000 + 2000 = 52000
        assert_eq!(cost, 52000);
    }

    #[test]
    fn cost_from_receipt_execution_only() {
        let receipt = TestReceipt { gas: 100, price: 5, blob_gas: None, blob_price: None };
        let cost = cost_from_receipt(&receipt);
        // exec_cost = 100 * 5 = 500
        // blob_cost = 0
        // total = 500
        assert_eq!(cost, 500);
    }

    #[test]
    fn cost_from_receipt_zero_cost() {
        let receipt = TestReceipt { gas: 0, price: 0, blob_gas: None, blob_price: None };
        let cost = cost_from_receipt(&receipt);
        assert_eq!(cost, 0);
    }

    #[test]
    fn cost_from_receipt_saturating_arithmetic() {
        let receipt = TestReceipt {
            gas: u64::MAX,
            price: u128::MAX,
            blob_gas: Some(u64::MAX),
            blob_price: Some(u128::MAX),
        };
        let cost = cost_from_receipt(&receipt);
        // Both calculations should saturate to u128::MAX, and the addition should also saturate
        assert_eq!(cost, u128::MAX);
    }
}
