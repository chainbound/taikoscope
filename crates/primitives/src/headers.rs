//! Block header data structures and stream type aliases.
use std::pin::Pin;

use alloy_primitives::{Address, BlockHash};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};

/// L1 Header
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct L1Header {
    /// Block number
    pub number: u64,
    /// Block hash
    pub hash: BlockHash,
    /// Block slot
    pub slot: u64,
    /// Extracted block timestamp
    pub timestamp: u64,
}

/// L2 Header
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct L2Header {
    /// Block number
    pub number: u64,
    /// Block hash
    pub hash: BlockHash,
    /// Block parent hash
    pub parent_hash: BlockHash,
    /// Block timestamp
    pub timestamp: u64,
    /// Gas used
    pub gas_used: u64,
    /// Beneficiary
    pub beneficiary: Address,
    /// Base fee per gas
    pub base_fee_per_gas: u64,
}

/// Stream of L1 headers
pub type L1HeaderStream = Pin<Box<dyn Stream<Item = L1Header> + Send>>;
/// Stream of L2 headers
pub type L2HeaderStream = Pin<Box<dyn Stream<Item = L2Header> + Send>>;
