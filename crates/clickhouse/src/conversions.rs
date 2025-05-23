use crate::models::{
    BatchRow, ForcedInclusionProcessedRow, L2HeadEvent, ProvedBatchRow, VerifiedBatchRow,
};
use chainio::{ITaikoInbox, taiko::wrapper::ITaikoWrapper};
use extractor::L2Header;
use eyre::{Context, Error, Result, eyre};
use std::convert::TryFrom;

// Conversion from L2Header to L2HeadEvent is intentionally omitted. The
// extractor provides additional block statistics which are used when
// constructing `L2HeadEvent`, so the direct conversion from a header would
// omit important values.

// Conversion from BatchProposed to BatchRow
impl TryFrom<&ITaikoInbox::BatchProposed> for BatchRow {
    type Error = Error;

    fn try_from(batch: &ITaikoInbox::BatchProposed) -> Result<Self, Self::Error> {
        let batch_size = u16::try_from(batch.info.blocks.len())?;
        let blob_count = u8::try_from(batch.info.blobHashes.len())?;

        let proposer_addr = batch.meta.proposer.into_array();

        Ok(Self {
            l1_block_number: batch.info.proposedIn,
            batch_id: batch.meta.batchId,
            batch_size,
            proposer_addr,
            blob_count,
            blob_total_bytes: batch.info.blobByteSize,
        })
    }
}

// Conversion from (BatchesProved, u64) to ProvedBatchRow
impl TryFrom<(&ITaikoInbox::BatchesProved, u64)> for ProvedBatchRow {
    type Error = Error;

    fn try_from(input: (&ITaikoInbox::BatchesProved, u64)) -> Result<Self, Self::Error> {
        let (proved, l1_block_number) = input;

        if proved.batchIds.is_empty() || proved.transitions.is_empty() {
            return Err(eyre!("Empty batch IDs or transitions"));
        }

        // For the example, we're just taking the first transition, but you might want to handle
        // all transitions in a real implementation
        let batch_id = proved.batchIds[0];
        let transition = &proved.transitions[0];
        let verifier_addr = proved.verifier.into_array();

        Ok(Self {
            l1_block_number,
            batch_id,
            verifier_addr,
            parent_hash: *transition.parentHash.as_ref(),
            block_hash: *transition.blockHash.as_ref(),
            state_root: *transition.stateRoot.as_ref(),
        })
    }
}

// Conversion from ForcedInclusionProcessed to ForcedInclusionProcessedRow
impl TryFrom<&ITaikoWrapper::ForcedInclusionProcessed> for ForcedInclusionProcessedRow {
    type Error = Error;

    fn try_from(event: &ITaikoWrapper::ForcedInclusionProcessed) -> Result<Self, Self::Error> {
        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(event.blobHash.as_slice());

        Ok(Self { blob_hash: hash_bytes })
    }
}

// Conversion from (BatchesVerified, u64) to VerifiedBatchRow
impl TryFrom<(&chainio::BatchesVerified, u64)> for VerifiedBatchRow {
    type Error = Error;

    fn try_from(input: (&chainio::BatchesVerified, u64)) -> Result<Self, Self::Error> {
        let (verified, l1_block_number) = input;

        Ok(Self { l1_block_number, batch_id: verified.batch_id, block_hash: verified.block_hash })
    }
}
