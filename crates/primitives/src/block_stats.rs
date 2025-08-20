use alloy_network_primitives::ReceiptResponse;
use alloy_primitives::Address;

/// Compute aggregated gas and priority fee statistics for a set of receipts,
/// excluding anchor transactions from gas and fee calculations.
///
/// Returns a tuple of `(total_gas_used, transaction_count, total_priority_fee)`.
/// The gas and fee totals exclude the anchor transaction, but the transaction
/// count includes all transactions (including the anchor).
#[allow(clippy::module_name_repetitions)]
pub fn compute_block_stats<R: ReceiptResponse>(
    receipts: &[R],
    base_fee: u64,
    anchor_address: Address,
) -> (u128, u32, u128) {
    let base = base_fee as u128;
    let mut sum_gas_used: u128 = 0;
    let mut sum_priority_fee: u128 = 0;

    for receipt in receipts {
        // Skip anchor transactions for gas and fee calculations
        if is_anchor_transaction(receipt, anchor_address) {
            continue;
        }

        let gas = receipt.gas_used() as u128;
        sum_gas_used += gas;
        let priority_per_gas = receipt.effective_gas_price().saturating_sub(base);
        sum_priority_fee += priority_per_gas.saturating_mul(gas);
    }

    // Transaction count includes all transactions (including anchor)
    let tx_count = receipts.len() as u32;
    (sum_gas_used, tx_count, sum_priority_fee)
}

/// Check if a receipt is for an anchor transaction.
fn is_anchor_transaction<R: ReceiptResponse>(receipt: &R, anchor_address: Address) -> bool {
    receipt.to().map(|addr| addr == anchor_address).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Address, B256, BlockHash, TxHash, address};

    const MAINNET_ANCHOR: Address = address!("1670000000000000000000000000000000001001");
    const HEKLA_ANCHOR: Address = address!("1670090000000000000000000000000000001001");

    #[derive(Debug, Clone)]
    struct TestReceipt {
        gas: u64,
        price: u128,
        to_addr: Option<Address>,
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
            self.to_addr
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
        let receipts = vec![
            TestReceipt { gas: 100, price: 10, to_addr: None },
            TestReceipt { gas: 200, price: 20, to_addr: None },
        ];
        let (gas, count, priority) = compute_block_stats(&receipts, 5, MAINNET_ANCHOR);
        assert_eq!(gas, 300);
        assert_eq!(count, 2);
        assert_eq!(priority, 3500);
    }

    #[test]
    fn compute_block_stats_zero_base_fee() {
        let receipts = vec![TestReceipt { gas: 150, price: 40, to_addr: None }];
        let (gas, count, priority) = compute_block_stats(&receipts, 0, MAINNET_ANCHOR);
        assert_eq!(gas, 150);
        assert_eq!(count, 1);
        assert_eq!(priority, 6000);
    }

    #[test]
    fn compute_block_stats_excludes_anchor() {
        let receipts = vec![
            // Anchor transaction - should be excluded
            TestReceipt { gas: 50, price: 10, to_addr: Some(MAINNET_ANCHOR) },
            // Regular transactions - should be included
            TestReceipt { gas: 100, price: 15, to_addr: None },
            TestReceipt { gas: 200, price: 20, to_addr: None },
        ];

        let (gas, count, priority) = compute_block_stats(&receipts, 10, MAINNET_ANCHOR);
        // Gas and fees exclude anchor, but count includes all transactions
        assert_eq!(gas, 300); // 100 + 200, excluding anchor's 50
        assert_eq!(count, 3); // All 3 transactions including anchor
        assert_eq!(priority, 2500); // (15-10)*100 + (20-10)*200 = 500 + 2000 = 2500
    }

    #[test]
    fn is_anchor_transaction_test() {
        let anchor_receipt = TestReceipt { gas: 50, price: 10, to_addr: Some(MAINNET_ANCHOR) };
        let regular_receipt = TestReceipt { gas: 100, price: 15, to_addr: None };

        assert!(is_anchor_transaction(&anchor_receipt, MAINNET_ANCHOR));
        assert!(!is_anchor_transaction(&regular_receipt, MAINNET_ANCHOR));
    }

    #[test]
    fn compute_block_stats_only_anchor() {
        let receipts = vec![TestReceipt { gas: 50, price: 10, to_addr: Some(MAINNET_ANCHOR) }];

        let (gas, count, priority) = compute_block_stats(&receipts, 10, MAINNET_ANCHOR);
        // Should count the transaction but exclude its gas and fees
        assert_eq!(gas, 0); // No gas counted (anchor excluded)
        assert_eq!(count, 1); // Transaction count still includes anchor
        assert_eq!(priority, 0); // No priority fees (anchor excluded)
    }

    #[test]
    fn compute_block_stats_different_anchor_addresses() {
        let mainnet_receipts = vec![
            TestReceipt { gas: 50, price: 10, to_addr: Some(MAINNET_ANCHOR) },
            TestReceipt { gas: 100, price: 15, to_addr: None },
        ];

        // Test with mainnet anchor address - should exclude mainnet anchor tx
        let (gas, count, _) = compute_block_stats(&mainnet_receipts, 10, MAINNET_ANCHOR);
        assert_eq!(gas, 100); // Only regular tx gas
        assert_eq!(count, 2); // Both transactions counted

        // Test with hekla anchor address - should NOT exclude mainnet anchor tx
        let (gas, count, _) = compute_block_stats(&mainnet_receipts, 10, HEKLA_ANCHOR);
        assert_eq!(gas, 150); // Both transactions' gas (50 + 100)
        assert_eq!(count, 2); // Both transactions counted

        let hekla_receipts = vec![
            TestReceipt { gas: 50, price: 10, to_addr: Some(HEKLA_ANCHOR) },
            TestReceipt { gas: 100, price: 15, to_addr: None },
        ];

        // Test with hekla anchor address - should exclude hekla anchor tx
        let (gas, count, _) = compute_block_stats(&hekla_receipts, 10, HEKLA_ANCHOR);
        assert_eq!(gas, 100); // Only regular tx gas
        assert_eq!(count, 2); // Both transactions counted
    }
}
