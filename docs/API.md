# Taikoscope API

This document lists the HTTP endpoints exposed by the Taikoscope API server. All responses are JSON encoded.

## Endpoints

| Path | Description |
| ---- | ----------- |
| `/l2-head` | Timestamp of the latest L2 block seen by the extractor. |
| `/l1-head` | Timestamp of the latest L1 block processed. |
| `/l2-head-block` | Latest L2 block number seen. |
| `/l1-head-block` | Latest L1 block number processed. |
| `/sse/l1-head` | Server-sent events stream of the latest L1 block number. |
| `/sse/l2-head` | Server-sent events stream of the latest L2 block number. |
| `/slashings` | List of slashing events. Optional `?range=24h/7d` query. |
| `/forced-inclusions` | Forced inclusion events. Optional `?range` query. |
| `/reorgs` | Detected L2 reorg events. Optional `?range` query. |
| `/active-gateways` | Gateways that have posted batches recently. Optional `?range` query. |
| `/current-operator` | Current operator address. |
| `/next-operator` | Next operator address. |
| `/avg-prove-time` | Average batch prove time. Optional `?range` query. |
| `/avg-verify-time` | Average batch verify time. Optional `?range` query. |
| `/l2-block-cadence` | Average time between L2 blocks. Optional `?range` query. |
| `/batch-posting-cadence` | Average time between batch submissions. Optional `?range` query. |
| `/avg-l2-tps` | Average transactions per second on L2. Optional `?range` query. |
| `/prove-times` | List of batches with their prove times. Optional `?range` query. |
| `/verify-times` | List of batches with their verify times. Optional `?range` query. |
| `/l1-block-times` | L1 block interval statistics. Optional `?range` query. |
| `/l2-block-times` | L2 block interval statistics. Optional `?range` query. |
| `/l2-gas-used` | Gas used per L2 block. Optional `?range` query. |
| `/sequencer-distribution` | Number of blocks produced by each sequencer. Optional `?range` query. |
| `/sequencer-blocks` | Blocks produced by each sequencer. Optional `?range` query. |
| `/block-transactions` | Number of transactions per L2 block. Supports `range`, `limit` and `offset` queries. |

Parameters such as `range` follow the form `1h`, `24h` or `7d` and default to one hour if omitted.
