# ReplacingMergeTree Migration Guide

This document describes the process for migrating `batches` and `batch_blocks` tables to ReplacingMergeTree for automatic deduplication.

## Overview

The migration is split into multiple phases for safety:

- **Migration 019**: ✅ Creates migration tracking system (automatic on startup)
- **Migration 020**: ✅ Creates ReplacingMergeTree shadow tables (automatic on startup)
- **Migration 021**: ⚠️ Data migration (MANUAL - requires maintenance window)
- **Migration 022**: ⚠️ Atomic table swap (MANUAL - requires maintenance window)  
- **Migration 023**: ✅ Cleanup old tables (automatic on startup, after manual steps)

## Automatic vs Manual Migrations

### Automatic (Safe for Startup)
- ✅ Migration 019: Creates `schema_migrations` tracking table
- ✅ Migration 020: Creates `batches_rmt` and `batch_blocks_rmt` tables
- ✅ Migration 023: Cleans up old backup tables

### Manual (Requires Maintenance Window)
- ⚠️ Migration 021: Copies and deduplicates data (time-intensive)
- ⚠️ Migration 022: Atomic table swap (requires application shutdown)

## Execution Plan

### Phase 1: Automatic Setup
1. Deploy application with new migrations
2. Migrations 019-020 run automatically on startup
3. Verify `batches_rmt` and `batch_blocks_rmt` tables are created

### Phase 2: Manual Data Migration (Maintenance Window)
1. **STOP APPLICATION** or put in read-only mode
2. Execute Migration 021 manually:
   ```bash
   clickhouse-client --database=taikoscope < migrations/021_migrate_to_replacing_merge_tree_MANUAL.sql
   ```
3. Verify data integrity (script includes verification queries)
4. Application can remain stopped or continue in read-only mode

### Phase 3: Manual Table Swap (Brief Downtime)
1. **STOP APPLICATION** completely
2. Execute Migration 022 manually:
   ```bash
   clickhouse-client --database=taikoscope < migrations/022_atomic_table_swap_MANUAL.sql
   ```
3. Verify swap completed successfully
4. **RESTART APPLICATION** 
5. Migration 023 will run automatically on startup to clean up

## Safety Features

### Rollback Plan
If issues occur after the table swap:
```sql
-- Rollback commands (included in migration 022)
RENAME TABLE taikoscope.batches TO taikoscope.batches_rmt_backup;
RENAME TABLE taikoscope.batch_blocks TO taikoscope.batch_blocks_rmt_backup;
RENAME TABLE taikoscope.batches_old TO taikoscope.batches;
RENAME TABLE taikoscope.batch_blocks_old TO taikoscope.batch_blocks;
```

### Data Integrity Verification
Each migration includes verification queries:
- Row counts match between old and new tables
- Sample data verification shows latest records were preserved
- Engine verification confirms ReplacingMergeTree is active

### Idempotency Protection
- Migration 021 checks for existing data before copying
- All operations use `IF NOT EXISTS` or `WHERE NOT EXISTS` clauses
- Migration tracking prevents duplicate execution

## Expected Timings

- **Migration 019-020**: < 1 second each
- **Migration 021**: 1-30 minutes (depends on data size)  
- **Migration 022**: < 1 second (atomic operations)
- **Migration 023**: < 1 second

## Monitoring

Check migration status:
```sql
SELECT * FROM taikoscope.schema_migrations ORDER BY applied_at DESC;
```

Verify table engines:
```sql
SELECT name, engine FROM system.tables 
WHERE database = 'taikoscope' 
  AND name IN ('batches', 'batch_blocks');
```

## Benefits After Migration

1. **Automatic Deduplication**: ReplacingMergeTree handles duplicates from gap detection/backfill
2. **Optimal Query Performance**: 
   - **Primary Key (`batch_id`)**: Fastest performance for most common JOINs
   - **Secondary Index (`inserted_at`)**: Fast monitoring queries (get_unproved/unverified_batches_older_than)
   - **Secondary Index (`l1_block_number`)**: Fast L1-based queries (get_batch_posting_*, get_last_batch_time)
3. **Monthly Partitioning**: Better data management and faster queries
4. **Preserved Projections**: `l2_block_number` lookups remain optimized

## Query Performance Impact

**Improved Performance:**
- `get_failed_proposals_since/range` - Fast batch_id JOINs
- `get_prove_times/paginated` - Fast batch_id JOINs

**Maintained Performance (via indexes):**
- `get_unproved_batches_older_than` - inserted_at index
- `get_unverified_batches_older_than` - inserted_at index  
- `get_last_batch_time` - l1_block_number index
- `get_batch_posting_cadence` - l1_block_number index
- `get_batch_posting_times/paginated` - l1_block_number index

**Result**: No performance degradation for any query pattern

## Questions?

This migration addresses the duplicate data issue while maintaining zero-downtime deployment compatibility. The startup migration strategy continues to work safely for future schema changes.