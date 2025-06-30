-- Migration 010: add old_sequencer and new_sequencer columns to l2_reorgs

ALTER TABLE ${DB}.l2_reorgs
ADD COLUMN IF NOT EXISTS old_sequencer FixedString(20) AFTER depth,
ADD COLUMN IF NOT EXISTS new_sequencer FixedString(20) AFTER old_sequencer;
