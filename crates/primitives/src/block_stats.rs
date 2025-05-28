use alloy_network_primitives::ReceiptResponse;

/// Compute aggregated gas and priority fee statistics for a set of receipts.
///
/// `base_fee` is optional and defaults to zero if not provided.
/// Returns a tuple of `(total_gas_used, transaction_count, total_priority_fee)`.
#[allow(clippy::module_name_repetitions)]
pub fn compute_block_stats<R: ReceiptResponse>(
    receipts: &[R],
    base_fee: Option<u64>,
) -> (u128, u32, u128) {
    let base = base_fee.unwrap_or(0) as u128;
    let mut sum_gas_used: u128 = 0;
    let mut sum_priority_fee: u128 = 0;

    for receipt in receipts {
        let gas = receipt.gas_used() as u128;
        sum_gas_used += gas;
        let priority_per_gas = receipt.effective_gas_price().saturating_sub(base);
        sum_priority_fee += priority_per_gas.saturating_mul(gas);
    }

    let tx_count = receipts.len() as u32;
    (sum_gas_used, tx_count, sum_priority_fee)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Address, B256, BlockHash, TxHash, address};

    #[derive(Debug, Clone, Copy)]
    struct TestReceipt {
        gas: u64,
        price: u128,
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
            None
        }
        fn blob_gas_price(&self) -> Option<u128> {
            None
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
    fn compute_block_stats_basic() {
        let receipts =
            vec![TestReceipt { gas: 100, price: 10 }, TestReceipt { gas: 200, price: 20 }];
        let (gas, count, priority) = compute_block_stats(&receipts, Some(5));
        assert_eq!(gas, 300);
        assert_eq!(count, 2);
        assert_eq!(priority, 3500);
    }

    #[test]
    fn compute_block_stats_zero_base_fee() {
        let receipts = vec![TestReceipt { gas: 150, price: 40 }];
        let (gas, count, priority) = compute_block_stats(&receipts, None);
        assert_eq!(gas, 150);
        assert_eq!(count, 1);
        assert_eq!(priority, 6000);
    }
}
