//! Reorg detection and handling functionality

use alloy_primitives::Address;
use clickhouse::{ClickhouseReader, ClickhouseWriter, HashBytes};
use extractor::ReorgDetector;
use tracing::{error, info, warn};

/// Process reorg detection for L2 headers
pub async fn process_reorg_detection(
    reorg_detector: &mut ReorgDetector,
    last_l2_header: &mut Option<(u64, Address)>,
    clickhouse_writer: &Option<ClickhouseWriter>,
    clickhouse_reader: &Option<ClickhouseReader>,
    header: &primitives::headers::L2Header,
) {
    let writer = match clickhouse_writer {
        Some(w) => w,
        None => return,
    };

    let old_head = reorg_detector.head_number();
    let reorg_result = reorg_detector.on_new_block_with_hash(header.number, header.hash);

    // Update last L2 header tracking
    *last_l2_header = Some((header.number, header.beneficiary));

    if let Some((depth, orphaned_hash)) = reorg_result {
        // Handle orphaned hash from one-block reorg
        if let Some(hash) = orphaned_hash {
            insert_orphaned_hash(writer, hash, header.number).await;
        }

        // Handle orphaned blocks from traditional reorg
        if depth > 0 {
            handle_traditional_reorg_orphans(
                writer,
                clickhouse_reader,
                old_head,
                header.number,
                depth,
            )
            .await;
        }

        // Process L2 reorg
        if let Some((prev_block_number, prev_sequencer)) = *last_l2_header {
            info!(
                prev_block = prev_block_number,
                new_block = header.number,
                prev_sequencer = ?prev_sequencer,
                new_sequencer = ?header.beneficiary,
                depth = depth,
                orphaned_hash = ?orphaned_hash,
                "L2 reorg detected"
            );

            // Insert L2 reorg record
            if let Err(e) = writer
                .insert_l2_reorg(header.number, depth, prev_sequencer, header.beneficiary)
                .await
            {
                error!(
                    block_number = header.number,
                    err = %e,
                    "Failed to insert L2 reorg record"
                );
            } else {
                info!(block_number = header.number, depth = depth, "Inserted L2 reorg record");
            }
        }
    }
}

/// Insert an orphaned hash from a one-block reorg
pub async fn insert_orphaned_hash(
    writer: &ClickhouseWriter,
    hash: alloy_primitives::B256,
    block_number: u64,
) {
    if let Err(e) = writer.insert_orphaned_hashes(&[(HashBytes::from(hash), block_number)]).await {
        error!(block_number, orphaned_hash = ?hash, err = %e, "Failed to insert orphaned hash");
    } else {
        info!(block_number, orphaned_hash = ?hash, "Inserted orphaned hash");
    }
}

/// Handle orphaned blocks from traditional reorgs
pub async fn handle_traditional_reorg_orphans(
    writer: &ClickhouseWriter,
    clickhouse_reader: &Option<ClickhouseReader>,
    old_head: u64,
    new_head: u64,
    depth: u16,
) {
    let orphaned_block_numbers = calculate_orphaned_blocks(old_head, new_head, depth.into());
    if orphaned_block_numbers.is_empty() {
        return;
    }

    let Some(reader) = clickhouse_reader else {
        return;
    };

    match reader.get_latest_hashes_for_blocks(&orphaned_block_numbers).await {
        Ok(orphaned_hashes) if !orphaned_hashes.is_empty() => {
            if let Err(e) = writer.insert_orphaned_hashes(&orphaned_hashes).await {
                error!(count = orphaned_hashes.len(), err = %e, "Failed to insert orphaned hashes");
            } else {
                info!(count = orphaned_hashes.len(), "Inserted orphaned hashes for reorg");
            }
        }
        Ok(_) => {} // No orphaned hashes found
        Err(e) => error!(err = %e, "Failed to fetch orphaned hashes"),
    }
}

/// Calculate which blocks are orphaned in a reorg
pub fn calculate_orphaned_blocks(old_head: u64, new_head: u64, depth: u32) -> Vec<u64> {
    // In a reorg, orphaned blocks are the blocks that were previously canonical
    // but are no longer on the main chain. These are the blocks from the fork point
    // back to the old head.

    if depth == 0 || old_head <= new_head {
        // No reorg or forward progression - no orphaned blocks
        return Vec::new();
    }

    // Calculate the fork point: old_head - depth + 1
    // Orphaned blocks are from fork_point to old_head (inclusive)
    let depth_u64 = depth as u64;
    if depth_u64 > old_head {
        // Edge case: depth is larger than old_head, return empty
        warn!(
            old_head = old_head,
            new_head = new_head,
            depth = depth,
            "Reorg depth exceeds old head block number"
        );
        return Vec::new();
    }

    let fork_point = old_head.saturating_sub(depth_u64) + 1;
    let orphaned_end = old_head + 1; // +1 because range is exclusive at end

    info!(
        old_head = old_head,
        new_head = new_head,
        depth = depth,
        fork_point = fork_point,
        orphaned_range = format!("{}..{}", fork_point, orphaned_end),
        "Calculating orphaned blocks for reorg"
    );

    (fork_point..orphaned_end).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_orphaned_blocks_no_reorg() {
        let result = calculate_orphaned_blocks(100, 100, 0);
        assert_eq!(result, vec![0u64; 0]);
    }

    #[test]
    fn test_calculate_orphaned_blocks_forward_progression() {
        let result = calculate_orphaned_blocks(100, 101, 1);
        assert_eq!(result, vec![0u64; 0]);
    }

    #[test]
    fn test_calculate_orphaned_blocks_simple_reorg() {
        let result = calculate_orphaned_blocks(100, 98, 2);
        assert_eq!(result, vec![99u64, 100]);
    }

    #[test]
    fn test_calculate_orphaned_blocks_deep_reorg() {
        let result = calculate_orphaned_blocks(100, 95, 5);
        assert_eq!(result, vec![96u64, 97, 98, 99, 100]);
    }

    #[test]
    fn test_calculate_orphaned_blocks_depth_exceeds_old_head() {
        let result = calculate_orphaned_blocks(10, 5, 15);
        assert_eq!(result, vec![0u64; 0]);
    }

    #[test]
    fn test_calculate_orphaned_blocks_edge_case_depth_equals_old_head() {
        let result = calculate_orphaned_blocks(10, 1, 10);
        assert_eq!(result, vec![1u64, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    }
}
