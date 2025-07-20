# Taikoscope Deployment Safety Guide

This document outlines safe deployment procedures for Taikoscope components to ensure zero event loss and no duplicate processing during deployments.

## Architecture Overview

```
┌─────────────┐    NATS     ┌─────────────┐    ClickHouse    ┌─────────────┐
│   Ingestor  │────────────▶│ JetStream   │────────────────▶│  Processor  │
│             │   Events    │  (Durable)  │    Events       │             │
└─────────────┘             └─────────────┘                 └─────────────┘
```

## Exactly-Once Delivery Guarantees

### NATS JetStream Configuration
- **Duplicate Window**: 120 seconds (configurable via `NATS_DUPLICATE_WINDOW_SECS`)
- **Storage**: File storage (configurable via `NATS_STORAGE_TYPE=file`)
- **Retention**: WorkQueue policy (configurable via `NATS_RETENTION_POLICY=workqueue`)
- **Consumer**: Durable consumer named "processor"

### Event Deduplication
Each event includes a unique `Msg-Id` header generated from:
- **L1 Headers**: `{number}:{hash}-l1_header`
- **L2 Headers**: `{number}:{hash}-l2_header`
- **Batch Events**: `{batch_id}:{l1_tx_hash}-{event_type}`
- **Forced Inclusion**: `{blobHash}-forced_inclusion_processed`

## Safe Deployment Procedures

### 1. Processor Deployment (Safe - No Event Loss Risk)

The processor can be safely offline for several minutes during deployment:

```bash
# 1. Stop the current processor
docker compose stop processor

# 2. Update the processor image
docker compose pull processor

# 3. Start the new processor
docker compose up -d processor
```

**Why this is safe:**
- NATS JetStream stores events in durable queues
- Durable consumer "processor" resumes from last acknowledged message
- No events are lost during processor downtime

### 2. Ingestor Deployment (Zero-Downtime with Overlap)

The ingestor requires a rolling deployment to prevent event loss:

```bash
# 1. Start new ingestor alongside the old one
docker compose up -d --scale ingestor=2 ingestor

# 2. Wait 30 seconds for new ingestor to fully start
sleep 30

# 3. Stop the old ingestor instance
docker compose stop ingestor_1

# 4. Scale back to single instance
docker compose up -d --scale ingestor=1 ingestor
```

**Why overlap is safe:**
- Both ingestors publish events with identical `Msg-Id` headers
- NATS deduplication (120s window) prevents duplicate processing
- No events are missed during the transition

### 3. NATS Deployment (Requires Data Persistence)

NATS should rarely need redeployment, but when it does:

**Prerequisites:**
- Ensure NATS data is persisted to EBS volume at `/mnt/data/nats`
- Update docker-compose.yml to include volume mapping

```yaml
services:
  nats:
    image: nats:latest
    volumes:
      - /mnt/data/nats:/data  # Persist JetStream data
    command: ["-js", "-m", "8222", "--store_dir", "/data"]
```

**Deployment procedure:**
```bash
# 1. Stop all consumers first
docker compose stop processor ingestor

# 2. Stop NATS
docker compose stop nats

# 3. Update and restart NATS
docker compose pull nats
docker compose up -d nats

# 4. Restart consumers
docker compose up -d processor ingestor
```

## Environment Variables for Production

### NATS Stream Configuration
```bash
# Production settings for exactly-once delivery
NATS_DUPLICATE_WINDOW_SECS=120
NATS_STORAGE_TYPE=file
NATS_RETENTION_POLICY=workqueue
```

### Monitoring Settings
```bash
# Enable all monitoring
INSTATUS_MONITORS_ENABLED=true
INSTATUS_MONITOR_POLL_INTERVAL_SECS=30
INSTATUS_MONITOR_THRESHOLD_SECS=600
```

## Deployment Verification

After any deployment, verify the system is working correctly:

### 1. Check Container Health
```bash
docker compose ps
docker compose logs processor --tail 50
docker compose logs ingestor --tail 50
```

### 2. Verify Event Processing
```bash
# Check processor logs for successful event processing
docker compose logs processor | grep "Inserted"

# Check ingestor logs for successful event publishing
docker compose logs ingestor | grep "Connected to NATS"
```

### 3. Monitor NATS Queue
```bash
# Access NATS monitoring interface
curl http://localhost:8222/jsz
```

### 4. Database Verification
```bash
# Check recent data in ClickHouse
# Connect to your ClickHouse instance and verify recent events
```

## Rollback Procedures

### Processor Rollback
```bash
# Use previous image tag
docker compose down processor
docker image tag taikoscope-processor:previous taikoscope-processor:latest
docker compose up -d processor
```

### Ingestor Rollback
```bash
# Quick rollback with overlap
docker compose up -d --scale ingestor=2 ingestor
# Wait for startup
docker image tag taikoscope-ingestor:previous taikoscope-ingestor:latest
docker compose up -d --scale ingestor=1 ingestor
```

## Troubleshooting

### Event Loss Detection
```bash
# Check for gaps in L1/L2 block numbers
# Monitor processor logs for "Failed to process message after all retries"
```

### Duplicate Event Detection
```bash
# Monitor NATS metrics for duplicate message rejections
curl http://localhost:8222/jsz | jq '.streams[].state.msgs'
```

### Connection Issues
```bash
# Check NATS connectivity
docker compose logs processor | grep "NATS connection health check"
docker compose logs ingestor | grep "Connected to NATS"
```

## Best Practices

1. **Always deploy processor first** - It's the safest component to restart
2. **Use ingestor overlap** - Never stop the ingestor without starting a replacement
3. **Monitor queue depth** - Watch for growing queues during deployments
4. **Test rollbacks** - Regularly test rollback procedures in staging
5. **Verify exactly-once** - Always check for duplicates after deployment
6. **Persist NATS data** - Ensure JetStream data survives container recreation

## Emergency Procedures

### Complete System Recovery
If all components fail:

1. **Restore NATS data** from EBS volume backup
2. **Start NATS** with persistent storage
3. **Start processor** to begin consuming queued events
4. **Start ingestor** to resume event ingestion
5. **Verify** no events were lost by checking block number continuity

### Data Verification
```bash
# Check for event continuity in ClickHouse
SELECT 
  min(l2_block_number) as min_block,
  max(l2_block_number) as max_block,
  count(*) as total_events,
  max_block - min_block + 1 as expected_events
FROM l2_headers 
WHERE l2_block_number > (SELECT max(l2_block_number) - 1000 FROM l2_headers);
```

This ensures bulletproof deployment safety with zero event loss and no duplicate processing.