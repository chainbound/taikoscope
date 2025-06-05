export function getMockData(path: string): unknown {
  switch (path) {
    case '/avg-prove-time':
      return { avg_prove_time_ms: 50 };
    case '/avg-verify-time':
      return { avg_verify_time_ms: 75 };
    case '/l2-block-cadence':
      return { l2_block_cadence_ms: 1000 };
    case '/batch-posting-cadence':
      return { batch_posting_cadence_ms: 2000 };
    case '/preconf-data':
      return {
        candidates: ['0xaaa', '0xbbb'],
        current_operator: '0xaaa',
        next_operator: '0xbbb',
      };
    case '/reorgs':
      return { events: [] };
    case '/slashings':
      return { events: [] };
    case '/forced-inclusions':
      return { events: [] };
    case '/l2-block-times':
      return {
        blocks: [
          { l2_block_number: 1, ms_since_prev_block: 100 },
          { l2_block_number: 2, ms_since_prev_block: 120 },
        ],
      };
    case '/l1-block-times':
      return {
        blocks: [
          { minute: 0, block_number: 1 },
          { minute: 60, block_number: 2 },
        ],
      };
    case '/sequencer-distribution':
      return {
        sequencers: [
          { address: '0xaaa', blocks: 10 },
          { address: '0xbbb', blocks: 20 },
        ],
      };
    case '/sequencer-blocks':
      return { sequencers: [{ address: '0xaaa', blocks: [1, 2, 3] }] };
    case '/block-transactions':
      return { blocks: [{ block: 1, txs: 2, sequencer: '0xaaa' }] };
    case '/blobs-per-batch':
      return { batches: [{ batch_id: 1, blob_count: 2 }] };
    case '/avg-blobs-per-batch':
      return { avg_blobs: 3 };
    case '/avg-l2-tps':
      return { avg_tps: 4 };
    case '/l2-tx-fee':
      return { tx_fee: 5 };
    case '/cloud-cost':
      return { cost_usd: 6 };
    case '/batch-posting-times':
      return { batches: [{ batch_id: 1, ms_since_prev_batch: 500 }] };
    case '/l2-gas-used':
      return { blocks: [{ l2_block_number: 1, gas_used: 1000 }] };
    case '/prove-times':
      return { batches: [{ batch_id: 1, seconds_to_prove: 12 }] };
    case '/verify-times':
      return { batches: [{ batch_id: 1, seconds_to_verify: 15 }] };
    case '/l2-head-block':
      return { l2_head_block: 42 };
    case '/l1-head-block':
      return { l1_head_block: 84 };
    default:
      return {};
  }
}
