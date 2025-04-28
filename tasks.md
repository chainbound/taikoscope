# Project Tasks

## Milestone 1: Project Initialization
- [x] Define crate structure and modules: `extractor`, `messaging`, `ingestion`, `schema`
- [x] Setup `Cargo.toml` with dependencies: `tokio`, `async-nats`, `clickhouse-rs`, `thiserror`, `anyhow`
- [x] Initialize GitHub Actions workflow for linting (Clippy), formatting, and tests

## Milestone 2: Messaging Infrastructure
- [ ] Implement NATS JetStream client module
  - [ ] Connect to NATS server
  - [ ] Publish head events with unique IDs
  - [ ] Subscribe with durable consumer
- [ ] Ensure exactly-once semantics via JetStream headers

## Milestone 3: Database Integration
- [ ] Implement ClickHouse client module
  - [x] Connect to ClickHouse
  - [ ] Define table schemas or migrations
  - [ ] Support batch insert operations

## Milestone 4: Extractor Development
- [ ] Implement L1 event extraction
  - [ ] Listen to Ethereum L1 head events
  - [ ] Fetch L1 block data and blob details
- [ ] Implement L2 event extraction
  - [ ] Listen to Taiko L2 head events
  - [ ] Fetch L2 block metrics (gas, tx count, priority fees)
- [ ] Handle slashing, forced inclusion, batch proposed, and block proven events

## Milestone 5: Inserter Development
- [ ] Consume messages from NATS
- [ ] Normalize and transform data into ClickHouse DTOs
- [ ] Insert data into corresponding tables with batch inserts
- [ ] Acknowledge NATS messages after successful insert

## Milestone 6: Error Handling & Reliability
- [ ] Define error types using `thiserror`/`anyhow`
- [ ] Implement retry policies and backoff for NATS and ClickHouse operations
- [ ] Handle extractor/inserter downtime and reorg detection

## Milestone 7: Testing & Quality
- [ ] Write unit tests for each module
- [ ] Write integration tests using test containers for NATS and ClickHouse
- [ ] Achieve at least 80% code coverage
- [ ] Setup coverage reporting in CI

## Milestone 8: Performance & Optimization
- [ ] Implement partitioning and projections/materialized views in ClickHouse
- [ ] Configure compression codecs
- [ ] Benchmark end-to-end throughput and tune backpressure

## Milestone 9: Deployment & Monitoring
- [ ] Dockerize services: extractor, inserter
- [ ] Setup GitHub Actions for Docker build and deployment
- [ ] Configure monitoring and alerting (metrics, logs)