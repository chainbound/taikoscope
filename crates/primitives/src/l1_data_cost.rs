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

/// Compute the total L1 data posting cost for the given transactions and receipts.
/// Only blob-carrying transactions sent to the provided inbox address are considered.
pub fn compute_l1_data_posting_cost<T, R>(txs: &[T], receipts: &[R], inbox: Address) -> u128
where
    T: Transaction,
    R: ReceiptResponse,
{
    let mut total = 0u128;
    for (tx, receipt) in txs.iter().zip(receipts) {
        if let Some(h) = tx.blob_versioned_hashes() {
            if !h.is_empty() && tx.to() == Some(inbox) {
                let gas_cost =
                    (receipt.gas_used() as u128).saturating_mul(receipt.effective_gas_price());
                let blob_fee = calculate_blob_fee_from_receipt(receipt);
                total = total.saturating_add(gas_cost.saturating_add(blob_fee));
            }
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
        assert_eq!(cost, 10 * 2 + 3 * 4);
    }

    #[test]
    fn ignores_non_matching_transactions() {
        let inbox = address!("0x000000000000000000000000000000000000dead");
        let tx = alloy_consensus::TxEip4844 {
            to: inbox,
            blob_versioned_hashes: vec![],
            ..Default::default()
        };
        let receipt = TestReceipt { gas: 10, price: 2, blob_gas: Some(3), blob_price: Some(4) };
        let cost = compute_l1_data_posting_cost(&[tx], &[receipt], inbox);
        assert_eq!(cost, 0);
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
}
