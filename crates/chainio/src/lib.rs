//! `ChainIO` is a library for interacting with on-chain contracts.
pub mod taiko;

use ITaikoInbox::{BatchProposed, BatchesProved, ITaikoInboxInstance, Transition};

use alloy::{
    primitives::Address,
    providers::{RootProvider, fillers::FillProvider, utils::JoinedRecommendedFillers},
    rpc::types::Filter,
    sol,
};
use derive_more::derive::Deref;

/// Alias to the default provider with all recommended fillers (read-only).
pub type DefaultProvider = FillProvider<JoinedRecommendedFillers, RootProvider>;

/// A wrapper over a `ITaikoInbox` contract that exposes various utility methods.
#[derive(Debug, Clone, Deref)]
pub struct TaikoInbox(ITaikoInboxInstance<DefaultProvider>);

impl TaikoInbox {
    /// Create a new `TaikoInbox` instance at the given contract address.
    pub const fn new_readonly(address: Address, provider: DefaultProvider) -> Self {
        Self(ITaikoInboxInstance::new(address, provider))
    }

    /// Returns a log [`Filter`] based on the `BatchProposed` event.
    pub fn batch_proposed_filter(&self) -> Filter {
        self.0.BatchProposed_filter().filter
    }
    /// Returns a log [`Filter`] based on the `BatchesProved` event.
    pub fn batches_proved_filter(&self) -> Filter {
        self.0.BatchesProved_filter().filter
    }
}

sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug)]
    interface ITaikoInbox {
        error AnchorBlockIdSmallerThanParent();
        error AnchorBlockIdTooLarge();
        error AnchorBlockIdTooSmall();
        error ArraySizesMismatch();
        error BatchNotFound();
        error BatchVerified();
        error BeyondCurrentFork();
        error BlobNotFound();
        error BlockNotFound();
        error BlobNotSpecified();
        error ContractPaused();
        error CustomProposerMissing();
        error CustomProposerNotAllowed();
        error EtherNotPaidAsBond();
        error FirstBlockTimeShiftNotZero();
        error ForkNotActivated();
        error InsufficientBond();
        error InvalidBlobCreatedIn();
        error InvalidBlobParams();
        error InvalidGenesisBlockHash();
        error InvalidParams();
        error InvalidTransitionBlockHash();
        error InvalidTransitionParentHash();
        error InvalidTransitionStateRoot();
        error MetaHashMismatch();
        error MsgValueNotZero();
        error NoBlocksToProve();
        error NotFirstProposal();
        error NotInboxWrapper();
        error ParentMetaHashMismatch();
        error SameTransition();
        error SignalNotSent();
        error TimestampSmallerThanParent();
        error TimestampTooLarge();
        error TimestampTooSmall();
        error TooManyBatches();
        error TooManyBlocks();
        error TooManySignals();
        error TransitionNotFound();
        error ZeroAnchorBlockHash();
        error Error(string);

        // These errors are from IPreconfRouter.sol, which extends ITaikoInbox.sol.
        error ForcedInclusionNotSupported();
        error NotFallbackPreconfer();
        error NotPreconfer();
        error ProposerIsNotPreconfer();

        // These errors are from TaikoWrapper.sol, which inherits most of ITaikoInbox.sol
        error InvalidBlockTxs();
        error InvalidBlobHashesSize();
        error InvalidBlobHash();
        error InvalidBlobByteOffset();
        error InvalidBlobByteSize();
        error InvalidBlockSize();
        error InvalidTimeShift();
        error InvalidSignalSlots();
        error OldestForcedInclusionDue();

        #[derive(Default)]
        event BatchProposed(BatchInfo info, BatchMetadata meta, bytes txList);
        #[derive(Default)]
        event BatchesProved(address verifier, uint64[] batchIds, Transition[] transitions);

        #[derive(Copy, Default)]
        struct BaseFeeConfig {
            uint8 adjustmentQuotient;
            uint8 sharingPctg;
            uint32 gasIssuancePerSecond;
            uint64 minGasExcess;
            uint32 maxGasIssuancePerBlock;
        }

        #[derive(Default)]
        struct BlockParams {
            // Number of transactions in the block
            uint16 numTransactions;
            // Time shift in seconds
            uint8 timeShift;
            // Signals sent on L1 and need to sync to this L2 block.
            bytes32[] signalSlots;
        }

        /// @dev This struct holds batch information essential for constructing blocks offchain, but it
        /// does not include data necessary for batch proving.
        #[derive(Default)]
        struct BatchInfo {
            bytes32 txsHash;
            // Data to build L2 blocks
            BlockParams[] blocks;
            bytes32[] blobHashes;
            bytes32 extraData;
            address coinbase;
            uint64 proposedIn; // Used by node/client
            uint64 blobCreatedIn;
            uint32 blobByteOffset;
            uint32 blobByteSize;
            uint32 gasLimit;
            uint64 lastBlockId;
            uint64 lastBlockTimestamp;
            // Data for the L2 anchor transaction, shared by all blocks in the batch
            uint64 anchorBlockId;
            // corresponds to the `_anchorStateRoot` parameter in the anchor transaction.
            // The batch's validity proof shall verify the integrity of these two values.
            bytes32 anchorBlockHash;
            BaseFeeConfig baseFeeConfig;
        }

        #[derive(Default)]
        struct BatchMetadata {
            bytes32 infoHash;
            address proposer;
            uint64 batchId;
            uint64 proposedAt; // Used by node/client
        }

        #[derive(Default)]
        struct Transition {
            bytes32 parentHash;
            bytes32 blockHash;
            bytes32 stateRoot;
        }

        #[derive(Default)]
        struct BlobParams {
            // The hashes of the blob. Note that if this array is not empty.  `firstBlobIndex` and
            // `numBlobs` must be 0.
            bytes32[] blobHashes;
            // The index of the first blob in this batch.
            uint8 firstBlobIndex;
            // The number of blobs in this batch. Blobs are initially concatenated and subsequently
            // decompressed via Zlib.
            uint8 numBlobs;
            // The byte offset of the blob in the batch.
            uint32 byteOffset;
            // The byte size of the blob.
            uint32 byteSize;
            // The block number when the blob was created.
            uint64 createdIn;
        }

        #[derive(Default)]
        struct BatchParams {
            address proposer;
            address coinbase;
            bytes32 parentMetaHash;
            uint64 anchorBlockId;
            uint64 lastBlockTimestamp;
            bool revertIfNotFirstProposal;
            // Specifies the number of blocks to be generated from this batch.
            BlobParams blobParams;
            BlockParams[] blocks;
        }

        #[derive(Default)]
        struct ForkHeights {
            uint64 ontake;
            uint64 pacaya;
        }

        /// @notice Struct holding Taiko configuration parameters. See {TaikoConfig}.
        /// NOTE: this was renamed from "Config" to "ProtocolConfig" for clarity.
        #[derive(Default)]
        struct ProtocolConfig {
            /// @notice The chain ID of the network where Taiko contracts are deployed.
            uint64 chainId;
            /// @notice The maximum number of unverified batches the protocol supports.
            uint64 maxUnverifiedBatches;
            /// @notice Size of the batch ring buffer, allowing extra space for proposals.
            uint64 batchRingBufferSize;
            /// @notice The maximum number of verifications allowed when a batch is proposed or proved.
            uint64 maxBatchesToVerify;
            /// @notice The maximum gas limit allowed for a block.
            uint32 blockMaxGasLimit;
            /// @notice The amount of Taiko token as a prover liveness bond per batch.
            uint96 livenessBondBase;
            /// @notice The amount of Taiko token as a prover liveness bond per block.
            uint96 livenessBondPerBlock;
            /// @notice The number of batches between two L2-to-L1 state root sync.
            uint8 stateRootSyncInternal;
            /// @notice The max differences of the anchor height and the current block number.
            uint64 maxAnchorHeightOffset;
            /// @notice Base fee configuration
            BaseFeeConfig baseFeeConfig;
            /// @notice The proving window in seconds.
            uint16 provingWindow;
            /// @notice The time required for a transition to be used for verifying a batch.
            uint24 cooldownWindow;
            /// @notice The maximum number of signals to be received by TaikoL2.
            uint8 maxSignalsToReceive;
            /// @notice The maximum number of blocks per batch.
            uint16 maxBlocksPerBatch;
            /// @notice Historical heights of the forks.
            ForkHeights forkHeights;
        }

        /// @notice 3 slots used.
        struct Batch {
            bytes32 metaHash; // slot 1
            uint64 lastBlockId; // slot 2
            uint96 reserved3;
            uint96 livenessBond;
            uint64 batchId; // slot 3
            uint64 lastBlockTimestamp;
            uint64 anchorBlockId;
            uint24 nextTransitionId;
            uint8 reserved4;
            // The ID of the transaction that is used to verify this batch. However, if this batch is
            // not verified as the last one in a transaction, verifiedTransitionId will remain zero.
            uint24 verifiedTransitionId;
        }

        struct Stats2 {
            uint64 numBatches;
            uint64 lastVerifiedBatchId;
            bool paused;
            uint56 lastProposedIn;
            uint64 lastUnpausedAt;
        }

        /// @notice Proposes a batch of blocks.
        /// @param _params ABI-encoded BlockParams.
        /// @param _txList The transaction list in calldata. If the txList is empty, blob will be used
        /// for data availability.
        /// @return info_ The info of the proposed batch.
        /// @return meta_ The metadata of the proposed batch.
        function proposeBatch(
            bytes calldata _params,
            bytes calldata _txList
        )
            external
            returns (BatchInfo memory info_, BatchMetadata memory meta_);

        /// @notice Retrieves the current protocol configuration.
        /// @return The current configuration.
        function pacayaConfig() external view returns (ProtocolConfig memory);

        /// @notice Retrieves the current stats2.
        /// @return The current stats2.
        function getStats2() external view returns (Stats2 memory);

        /// @notice Retrieves a batch by its ID.
        /// @param batchId The ID of the batch to retrieve.
        /// @return The batch.
        function getBatch(uint64 batchId) public view returns (Batch memory);
    }
}

impl BatchProposed {
    /// Returns the block numbers that were proposed in this batch, by looking
    /// at the `info.blocks` and `lastBlockId` fields.
    pub fn block_numbers_proposed(&self) -> Vec<u64> {
        let last = self.info.lastBlockId;

        let count = self.info.blocks.len() as u64;

        // Add 1 to avoid off-by-one errors.
        // Example: `last == 3`, `count == 3`, then `first == 1`.
        let first = last.saturating_sub(count) + 1;

        (first..=last).collect()
    }

    /// Returns the last block number proposed in this batch.
    pub const fn last_block_number(&self) -> u64 {
        self.info.lastBlockId
    }

    /// Returns the last block timestamp proposed in this batch.
    pub const fn last_block_timestamp(&self) -> u64 {
        self.info.lastBlockTimestamp
    }
}

impl BatchesProved {
    /// Returns the batch IDs proved in this event.
    pub fn batch_ids_proved(&self) -> &[u64] {
        &self.batchIds
    }

    /// Returns the transitions proved in this event.
    pub fn transitions_proved(&self) -> &[Transition] {
        &self.transitions
    }
}

/// Struct for handling `BatchesVerified` events
#[derive(Debug, Default)]
pub struct BatchesVerified {
    /// Batch ID that was verified
    pub batch_id: u64,
    /// Block hash of the verified batch
    pub block_hash: [u8; 32],
}

impl BatchesVerified {
    /// Returns the batch ID that was verified
    pub const fn batch_id(&self) -> u64 {
        self.batch_id
    }

    /// Returns the block hash of the verified batch
    pub const fn block_hash(&self) -> &[u8; 32] {
        &self.block_hash
    }
}
