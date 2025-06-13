use alloy_network_primitives::ReceiptResponse;

/// Returns the total fee paid for blobs in the given receipt.
/// If the receipt is _not_ a blob-carrying transaction, `0` is returned.
pub fn calculate_blob_fee_from_receipt<R: ReceiptResponse>(receipt: &R) -> u128 {
    match (receipt.blob_gas_used(), receipt.blob_gas_price()) {
        (Some(gas_used), Some(price)) => (gas_used as u128).saturating_mul(price),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Address, B256, BlockHash, TxHash, address};

    #[derive(Debug, Clone, Copy)]
    struct TestReceipt {
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
            0
        }
        fn effective_gas_price(&self) -> u128 {
            0
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
            0
        }
        fn state_root(&self) -> Option<B256> {
            None
        }
    }

    #[test]
    fn test_blob_fee_calculation() {
        let receipt = TestReceipt { blob_gas: Some(100), blob_price: Some(10) };
        assert_eq!(calculate_blob_fee_from_receipt(&receipt), 1000);
    }

    #[test]
    fn test_no_blob_fee() {
        let receipt = TestReceipt { blob_gas: None, blob_price: Some(10) };
        assert_eq!(calculate_blob_fee_from_receipt(&receipt), 0);

        let receipt = TestReceipt { blob_gas: Some(100), blob_price: None };
        assert_eq!(calculate_blob_fee_from_receipt(&receipt), 0);

        let receipt = TestReceipt { blob_gas: None, blob_price: None };
        assert_eq!(calculate_blob_fee_from_receipt(&receipt), 0);
    }

    #[test]
    fn test_saturating_mul() {
        let receipt = TestReceipt { blob_gas: Some(u64::MAX), blob_price: Some(u128::MAX) };
        assert_eq!(calculate_blob_fee_from_receipt(&receipt), u128::MAX);
    }
}
