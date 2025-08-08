-- Migration 016: Add data skipping indices to improve common equality/IN filters
-- Strategy: Prefer bloom_filter(0.01) for FixedString and numeric equality lookups.
-- After adding an index, MATERIALIZE it so it is built for existing data parts.

-- l1_head_events: lookup by block_hash
ALTER TABLE ${DB}.l1_head_events
    ADD INDEX idx_l1_block_hash_bf block_hash TYPE bloom_filter(0.01) GRANULARITY 1;
ALTER TABLE ${DB}.l1_head_events MATERIALIZE INDEX idx_l1_block_hash_bf;

-- l2_head_events: lookup by block_hash and sequencer
ALTER TABLE ${DB}.l2_head_events
    ADD INDEX idx_l2_block_hash_bf block_hash TYPE bloom_filter(0.01) GRANULARITY 1,
    ADD INDEX idx_l2_sequencer_bf  sequencer  TYPE bloom_filter(0.01) GRANULARITY 1;
ALTER TABLE ${DB}.l2_head_events MATERIALIZE INDEX idx_l2_block_hash_bf;
ALTER TABLE ${DB}.l2_head_events MATERIALIZE INDEX idx_l2_sequencer_bf;

-- batches: access by batch_id, l1_tx_hash, proposer address
ALTER TABLE ${DB}.batches
    ADD INDEX idx_batches_batch_id_bf  batch_id   TYPE bloom_filter(0.01) GRANULARITY 1,
    ADD INDEX idx_batches_l1_tx_bf     l1_tx_hash TYPE bloom_filter(0.01) GRANULARITY 1,
    ADD INDEX idx_batches_proposer_bf  proposer_addr TYPE bloom_filter(0.01) GRANULARITY 1;
ALTER TABLE ${DB}.batches MATERIALIZE INDEX idx_batches_batch_id_bf;
ALTER TABLE ${DB}.batches MATERIALIZE INDEX idx_batches_l1_tx_bf;
ALTER TABLE ${DB}.batches MATERIALIZE INDEX idx_batches_proposer_bf;

-- batch_blocks: accelerate block -> batch lookups
ALTER TABLE ${DB}.batch_blocks
    ADD INDEX idx_batch_blocks_l2_block_bf l2_block_number TYPE bloom_filter(0.01) GRANULARITY 1;
ALTER TABLE ${DB}.batch_blocks MATERIALIZE INDEX idx_batch_blocks_l2_block_bf;

-- proved_batches: lookups by batch_id and block_hash
ALTER TABLE ${DB}.proved_batches
    ADD INDEX idx_proved_batches_batch_id_bf batch_id   TYPE bloom_filter(0.01) GRANULARITY 1,
    ADD INDEX idx_proved_batches_block_bf    block_hash TYPE bloom_filter(0.01) GRANULARITY 1;
ALTER TABLE ${DB}.proved_batches MATERIALIZE INDEX idx_proved_batches_batch_id_bf;
ALTER TABLE ${DB}.proved_batches MATERIALIZE INDEX idx_proved_batches_block_bf;

-- verified_batches: lookups by batch_id and block_hash
ALTER TABLE ${DB}.verified_batches
    ADD INDEX idx_verified_batches_batch_id_bf batch_id   TYPE bloom_filter(0.01) GRANULARITY 1,
    ADD INDEX idx_verified_batches_block_bf    block_hash TYPE bloom_filter(0.01) GRANULARITY 1;
ALTER TABLE ${DB}.verified_batches MATERIALIZE INDEX idx_verified_batches_batch_id_bf;
ALTER TABLE ${DB}.verified_batches MATERIALIZE INDEX idx_verified_batches_block_bf;

-- slashing_events: filter by validator address
ALTER TABLE ${DB}.slashing_events
    ADD INDEX idx_slashing_validator_bf validator_addr TYPE bloom_filter(0.01) GRANULARITY 1;
ALTER TABLE ${DB}.slashing_events MATERIALIZE INDEX idx_slashing_validator_bf;

-- l1_data_costs: filter by batch_id
ALTER TABLE ${DB}.l1_data_costs
    ADD INDEX idx_l1_data_costs_batch_id_bf batch_id TYPE bloom_filter(0.01) GRANULARITY 1;
ALTER TABLE ${DB}.l1_data_costs MATERIALIZE INDEX idx_l1_data_costs_batch_id_bf;

-- prove_costs / verify_costs: filter by batch_id
ALTER TABLE ${DB}.prove_costs
    ADD INDEX idx_prove_costs_batch_id_bf batch_id TYPE bloom_filter(0.01) GRANULARITY 1;
ALTER TABLE ${DB}.prove_costs MATERIALIZE INDEX idx_prove_costs_batch_id_bf;

ALTER TABLE ${DB}.verify_costs
    ADD INDEX idx_verify_costs_batch_id_bf batch_id TYPE bloom_filter(0.01) GRANULARITY 1;
ALTER TABLE ${DB}.verify_costs MATERIALIZE INDEX idx_verify_costs_batch_id_bf;

-- orphaned_l2_hashes: lookups by block_hash
ALTER TABLE ${DB}.orphaned_l2_hashes
    ADD INDEX idx_orphaned_l2_block_hash_bf block_hash TYPE bloom_filter(0.01) GRANULARITY 1;
ALTER TABLE ${DB}.orphaned_l2_hashes MATERIALIZE INDEX idx_orphaned_l2_block_hash_bf;

-- preconf_data: filters by operator fields
ALTER TABLE ${DB}.preconf_data
    ADD INDEX idx_preconf_current_op_bf current_operator TYPE bloom_filter(0.01) GRANULARITY 1,
    ADD INDEX idx_preconf_next_op_bf    next_operator    TYPE bloom_filter(0.01) GRANULARITY 1;
ALTER TABLE ${DB}.preconf_data MATERIALIZE INDEX idx_preconf_current_op_bf;
ALTER TABLE ${DB}.preconf_data MATERIALIZE INDEX idx_preconf_next_op_bf;

-- l2_reorgs: common filters by l2_block_number and sequencers
ALTER TABLE ${DB}.l2_reorgs
    ADD INDEX idx_l2_reorgs_block_num_bf l2_block_number TYPE bloom_filter(0.01) GRANULARITY 1,
    ADD INDEX idx_l2_reorgs_old_seq_bf   old_sequencer   TYPE bloom_filter(0.01) GRANULARITY 1,
    ADD INDEX idx_l2_reorgs_new_seq_bf   new_sequencer   TYPE bloom_filter(0.01) GRANULARITY 1;
ALTER TABLE ${DB}.l2_reorgs MATERIALIZE INDEX idx_l2_reorgs_block_num_bf;
ALTER TABLE ${DB}.l2_reorgs MATERIALIZE INDEX idx_l2_reorgs_old_seq_bf;
ALTER TABLE ${DB}.l2_reorgs MATERIALIZE INDEX idx_l2_reorgs_new_seq_bf;

-- forced_inclusion_processed: lookup by blob_hash
ALTER TABLE ${DB}.forced_inclusion_processed
    ADD INDEX idx_forced_inclusion_blob_bf blob_hash TYPE bloom_filter(0.01) GRANULARITY 1;
ALTER TABLE ${DB}.forced_inclusion_processed MATERIALIZE INDEX idx_forced_inclusion_blob_bf;
