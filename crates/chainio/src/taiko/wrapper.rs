//! Taiko wrapper contract
use alloy::{
    providers::ProviderBuilder,
    rpc::{client::ClientBuilder, types::Filter},
};
use alloy_primitives::Address;
use alloy_sol_macro::sol;
use derive_more::derive::Deref;
use url::Url;

use crate::DefaultProvider;

use ITaikoWrapper::ITaikoWrapperInstance;

/// A wrapper around the `TaikoWrapper` contract.
#[derive(Debug, Clone, Deref)]
pub struct TaikoWrapper(ITaikoWrapperInstance<DefaultProvider>);

impl TaikoWrapper {
    /// Create a new `TaikoWrapper` instance at the given contract address.
    pub fn from_address<U: Into<Url>>(el_client_url: U, address: Address) -> Self {
        let client = ClientBuilder::default().http(el_client_url.into());
        let provider = ProviderBuilder::new().connect_client(client);
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
    #[derive(Debug)]
    interface IForcedInclusionStore {
        struct ForcedInclusion {
            bytes32 blobHash;
            uint64 feeInGwei;
            uint64 createdAtBatchId;
            uint32 blobByteOffset;
            uint32 blobByteSize;
            uint64 blobCreatedIn;
        }
    }

    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug)]
    interface ITaikoWrapper {
        struct ForcedInclusion {
            bytes32 blobHash;
            uint64 feeInGwei;
            uint64 createdAtBatchId;
            uint32 blobByteOffset;
            uint32 blobByteSize;
            uint64 blobCreatedIn;
        }

        event ForcedInclusionProcessed(ForcedInclusion forcedInclusion);
    }
}
