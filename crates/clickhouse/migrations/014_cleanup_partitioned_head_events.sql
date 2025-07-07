-- Migration 014: Drop old shadow tables for head_events now that everyone's on the partitioned tables
DROP TABLE IF EXISTS ${DB}.l2_head_events_old;
DROP TABLE IF EXISTS ${DB}.l1_head_events_old;