-- Migration 003: Add missing sum_base_fee column to l2_head_events table
-- This migration adds the sum_base_fee column that was defined in the original schema
-- but appears to be missing from the actual database table

ALTER TABLE ${DB}.l2_head_events 
ADD COLUMN IF NOT EXISTS sum_base_fee UInt128;