//! Taiko wrapper contract
use alloy::rpc::types::Filter;
use alloy_primitives::Address;
use alloy_sol_macro::sol;
use derive_more::derive::Deref;

use crate::DefaultProvider;

use ITaikoWrapper::ITaikoWrapperInstance;

/// A wrapper around the `TaikoWrapper` contract.
#[derive(Debug, Clone, Deref)]
pub struct TaikoWrapper(ITaikoWrapperInstance<DefaultProvider>);

impl TaikoWrapper {
    /// Create a new `TaikoWrapper` instance over an existing WS-based provider.
    pub const fn new_readonly(address: Address, provider: DefaultProvider) -> Self {
        Self(ITaikoWrapperInstance::new(address, provider))
    }

    /// Returns a log [`Filter`] based on the `ForcedInclusionProcessed` event.
    pub fn forced_inclusion_processed_filter(&self) -> Filter {
        self.0.ForcedInclusionProcessed_filter().filter
    }
}

sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    interface ITaikoWrapper {
        struct ForcedInclusion {
            bytes32 blobHash;
            uint64 feeInGwei;
            uint64 createdAtBatchId;
            uint32 blobByteOffset;
            uint32 blobByteSize;
            uint64 blobCreatedIn;
        }

        event ForcedInclusionProcessed(
            ForcedInclusion forcedInclusion
        );
    }
}
