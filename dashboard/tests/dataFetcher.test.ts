import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  fetchMainDashboardData,
  fetchEconomicsData,
} from '../utils/dataFetcher';
import * as api from '../services/apiService.ts';

type Res<T> = { data: T; badRequest: boolean; error: null };
const ok = <T>(data: T): Res<T> => ({ data, badRequest: false, error: null });

// helper to set all mocks to return provided data
function setAll(data: Partial<Record<keyof typeof api, unknown>>) {
  for (const [key, value] of Object.entries(data)) {
    // @ts-expect-error - Dynamic property access for test mocking
    vi.spyOn(api, key as keyof typeof api).mockResolvedValue(value);
  }
}

beforeEach(() => {
  vi.restoreAllMocks();
});

describe('dataFetcher', () => {
  it('aggregates main dashboard data', async () => {
    setAll({
      fetchDashboardData: ok({
        l2_block_cadence_ms: 1,
        batch_posting_cadence_ms: 2,
        avg_prove_time_ms: 3,
        avg_verify_time_ms: 4,
        avg_tps: 5,
        preconf_data: {
          candidates: ['a'],
          current_operator: 'x',
          next_operator: 'y',
        },
        l2_reorgs: 6,
        slashings: 7,
        forced_inclusions: 8,
        l2_block: 10,
        l1_block: 11,
        l2_tx_fee: 12,
        cloud_cost: 13,
      }),
      fetchProveTimes: ok([{ name: '1', value: 1, timestamp: 0 }]),
      fetchVerifyTimes: ok([{ name: '2', value: 2, timestamp: 0 }]),
      fetchL2BlockTimes: ok([{ value: 2, timestamp: 0 }]),
      fetchL2GasUsed: ok([{ value: 3, timestamp: 0 }]),
      fetchSequencerDistribution: ok([{ name: 'foo', value: 1 }]),
      fetchAllBlockTransactions: ok([{ block: 1, txs: 2, sequencer: 'bar' }]),
      fetchBatchBlobCounts: ok([{ block: 10, batch: 1, blobs: 2 }]),
    });

    const res = await fetchMainDashboardData('1h', null);
    expect(res.avgProve).toBe(3);
    expect(res.avgVerify).toBe(4);
    expect(res.sequencerDist[0].name).toBe('foo');
    expect(res.txPerBlock).toHaveLength(1);
    expect(res.badRequestResults).toHaveLength(8);
  });

  it('defaults to empty arrays when service data missing', async () => {
    setAll({
      fetchDashboardData: ok(null),
      fetchProveTimes: ok(null),
      fetchVerifyTimes: ok(null),
      fetchL2BlockTimes: ok(null),
      fetchL2GasUsed: ok(null),
      fetchSequencerDistribution: ok(null),
      fetchAllBlockTransactions: ok(null),
      fetchBatchBlobCounts: ok(null),
    });

    const res = await fetchMainDashboardData('1h', null);
    expect(res.proveTimes).toEqual([]);
    expect(res.sequencerDist).toEqual([]);
    expect(res.txPerBlock).toEqual([]);
    expect(res.blobsPerBatch).toEqual([]);
    expect(res.badRequestResults).toHaveLength(8);
  });

  it('fetches economics data', async () => {
    setAll({
      fetchL2TxFee: ok(1),
      fetchL2HeadBlock: ok(2),
      fetchL1HeadBlock: ok(3),
    });

    const res = await fetchEconomicsData('1h', null);
    expect(res.l2TxFee).toBe(1);
    expect(res.l2Block).toBe(2);
    expect(res.l1Block).toBe(3);
    expect(res.badRequestResults).toHaveLength(3);
  });
});
