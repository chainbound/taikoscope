import { describe, it, expect, afterEach } from 'vitest';

import {
  fetchAvgProveTime,
  fetchActiveGateways,
  fetchActiveGatewayAddresses,
  fetchL2BlockTimes,
  fetchBlockTransactions,
  fetchAvgL2Tps,
} from '../services/apiService.ts';

const originalFetch = globalThis.fetch;

// helper to create mock fetch response
function mockFetch(data: unknown, status = 200, ok = true) {
  return async () =>
    ({
      ok,
      status,
      json: async () => data,
    }) as Response;
}

describe('apiService', () => {
  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it('fetchAvgProveTime succeeds', async () => {
    globalThis.fetch = mockFetch({ avg_prove_time_ms: 42 });
    const prove = await fetchAvgProveTime('1h');
    expect(prove.badRequest).toBe(false);
    expect(prove.data).toBe(42);
  });

  it('handles bad request for fetchAvgProveTime', async () => {
    globalThis.fetch = mockFetch({}, 400, false);
    const badProve = await fetchAvgProveTime('1h');
    expect(badProve.badRequest).toBe(true);
    expect(badProve.data).toBeNull();
  });

  it('transforms active gateways', async () => {
    globalThis.fetch = mockFetch({ gateways: ['a', 'b'] });
    const gateways = await fetchActiveGateways('1h');
    expect(gateways.data).toBe(2);
  });

  it('fetches active gateway addresses', async () => {
    globalThis.fetch = mockFetch({ gateways: ['a', 'b'] });
    const gateways = await fetchActiveGatewayAddresses('1h');
    expect(gateways.data).toStrictEqual(['a', 'b']);
  });

  it('transforms block times', async () => {
    globalThis.fetch = mockFetch({
      blocks: [
        { l2_block_number: 1, ms_since_prev_block: 10 },
        { l2_block_number: 2, ms_since_prev_block: 20 },
      ],
    });
    const blockTimes = await fetchL2BlockTimes('1h');
    expect(blockTimes.data).toStrictEqual([{ value: 2, timestamp: 20 }]);
  });

  it('transforms block transactions', async () => {
    globalThis.fetch = mockFetch({
      blocks: [{ block: 1, txs: 3, sequencer: '0xabc' }],
    });
    const txs = await fetchBlockTransactions('1h');
    expect(txs.data).toStrictEqual([{ block: 1, txs: 3, sequencer: '0xabc' }]);
  });

  it('fetchAvgL2Tps succeeds', async () => {
    globalThis.fetch = mockFetch({ avg_tps: 1.5 });
    const res = await fetchAvgL2Tps('1h');
    expect(res.badRequest).toBe(false);
    expect(res.data).toBe(1.5);
  });

  it('handles bad request for fetchAvgL2Tps', async () => {
    globalThis.fetch = mockFetch({}, 400, false);
    const res = await fetchAvgL2Tps('1h');
    expect(res.badRequest).toBe(true);
    expect(res.data).toBeNull();
  });
});
