// Generate realistic mock data with proper time series and varied values
const generateTimeBasedBlocks = (count: number, startBlockNumber: number) => {
  const now = Date.now();
  const blocks = [];
  for (let i = 0; i < count; i++) {
    blocks.push({
      l2_block_number: startBlockNumber + i,
      ms_since_prev_block: 800 + Math.random() * 400, // 800-1200ms variance
    });
  }
  return blocks;
};

const generateL1Blocks = (count: number, intervalMinutes: number) => {
  const blocks = [];
  for (let i = 0; i < count; i++) {
    blocks.push({
      minute: i * intervalMinutes,
      block_number: 18500000 + i,
    });
  }
  return blocks;
};

const generateSequencerAddresses = () => [
  '0x1234567890123456789012345678901234567890',
  '0xabcdefabcdefabcdefabcdefabcdefabcdefabcd',
  '0x9876543210987654321098765432109876543210',
  '0xfedcbafedcbafedcbafedcbafedcbafedcbafedcba',
  '0x1111222233334444555566667777888899990000',
];

const mockSequencers = generateSequencerAddresses();

export function getMockData(path: string): unknown {
  switch (path) {
    case '/avg-prove-time':
      return { avg_prove_time_ms: 45000 + Math.random() * 10000 }; // 45-55 seconds
    case '/avg-verify-time':
      return { avg_verify_time_ms: 25000 + Math.random() * 5000 }; // 25-30 seconds
    case '/l2-block-cadence':
      return { l2_block_cadence_ms: 950 + Math.random() * 100 }; // ~1 second
    case '/batch-posting-cadence':
      return { batch_posting_cadence_ms: 1800000 + Math.random() * 400000 }; // ~30-35 minutes
    case '/preconf-data':
      return {
        candidates: mockSequencers,
        current_operator: mockSequencers[0],
        next_operator: mockSequencers[1],
      };
    case '/reorgs':
      return {
        events: [
          {
            l2_block_number: 12345,
            depth: 2,
            inserted_at: new Date(Date.now() - 3600000).toISOString(),
          },
          {
            l2_block_number: 12290,
            depth: 1,
            inserted_at: new Date(Date.now() - 7200000).toISOString(),
          },
        ],
      };
    case '/slashings':
      return {
        events: [
          {
            l1_block_number: 18500123,
            validator_addr: [1, 2, 3, 4, 5],
          },
        ],
      };
    case '/forced-inclusions':
      return {
        events: [
          {
            blob_hash: [10, 20, 30, 40, 50],
          },
          {
            blob_hash: [60, 70, 80, 90, 100],
          },
        ],
      };
    case '/l2-block-times':
      return {
        blocks: generateTimeBasedBlocks(50, 12300),
      };
    case '/l1-block-times':
      return {
        blocks: generateL1Blocks(24, 1), // 24 hours of data
      };
    case '/sequencer-distribution':
      return {
        sequencers: [
          { address: mockSequencers[0], blocks: 1250 },
          { address: mockSequencers[1], blocks: 980 },
          { address: mockSequencers[2], blocks: 1100 },
          { address: mockSequencers[3], blocks: 750 },
          { address: mockSequencers[4], blocks: 890 },
        ],
      };
    case '/sequencer-blocks':
      return {
        sequencers: mockSequencers.map((addr, i) => ({
          address: addr,
          blocks: Array.from({ length: 20 + i * 5 }, (_, j) => 12000 + i * 100 + j),
        })),
      };
    case '/block-transactions':
      return {
        blocks: Array.from({ length: 50 }, (_, i) => ({
          block: 12300 + i,
          txs: Math.floor(Math.random() * 200) + 50, // 50-250 transactions
          sequencer: mockSequencers[i % mockSequencers.length],
        })),
      };
    case '/blobs-per-batch':
      return {
        batches: Array.from({ length: 30 }, (_, i) => ({
          batch_id: 8800 + i,
          blob_count: Math.floor(Math.random() * 6) + 1, // 1-6 blobs
        })),
      };
    case '/avg-blobs-per-batch':
      return { avg_blobs: 3.2 + Math.random() * 1.5 }; // 3.2-4.7 average
    case '/avg-l2-tps':
      return { avg_tps: 120 + Math.random() * 80 }; // 120-200 TPS
    case '/l2-tx-fee':
      return { tx_fee: 0.000012 + Math.random() * 0.000008 }; // 0.000012-0.00002 ETH
    case '/cloud-cost':
      return { cost_usd: 1200 + Math.random() * 300 }; // $1200-1500
    case '/batch-posting-times':
      return {
        batches: Array.from({ length: 20 }, (_, i) => ({
          batch_id: 8800 + i,
          ms_since_prev_batch: 1800000 + Math.random() * 600000, // 30-40 minutes
        })),
      };
    case '/l2-gas-used':
      return {
        blocks: Array.from({ length: 40 }, (_, i) => ({
          l2_block_number: 12300 + i,
          gas_used: Math.floor(Math.random() * 8000000) + 2000000, // 2M-10M gas
        })),
      };
    case '/prove-times':
      return {
        batches: Array.from({ length: 25 }, (_, i) => ({
          batch_id: 8800 + i,
          seconds_to_prove: 30 + Math.random() * 30, // 30-60 seconds
        })),
      };
    case '/verify-times':
      return {
        batches: Array.from({ length: 25 }, (_, i) => ({
          batch_id: 8800 + i,
          seconds_to_verify: 15 + Math.random() * 20, // 15-35 seconds
        })),
      };
    case '/l2-head-block':
      return { l2_head_block: 12349 };
    case '/l1-head-block':
      return { l1_head_block: 18500024 };
    default:
      return {};
  }
}
