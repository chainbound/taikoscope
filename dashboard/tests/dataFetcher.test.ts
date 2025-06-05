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
    // @ts-ignore
    vi.spyOn(api, key as any).mockResolvedValue(value);
  }
}

beforeEach(() => {
  vi.restoreAllMocks();
});

describe('dataFetcher', () => {
  it('aggregates main dashboard data', async () => {
    setAll({
      fetchL2BlockCadence: ok(1),
      fetchBatchPostingCadence: ok(2),
      fetchAvgProveTime: ok(3),
      fetchAvgVerifyTime: ok(4),
      fetchAvgL2Tps: ok(5),
      fetchPreconfData: ok({
        candidates: ['a'],
        current_operator: 'x',
        next_operator: 'y',
      }),
      fetchL2Reorgs: ok(6),
      fetchSlashingEventCount: ok(7),
      fetchForcedInclusionCount: ok(8),
      fetchL2HeadBlock: ok(10),
      fetchL1HeadBlock: ok(11),
      fetchProveTimes: ok([{ name: '1', value: 1, timestamp: 0 }]),
      fetchVerifyTimes: ok([{ name: '2', value: 2, timestamp: 0 }]),
      fetchL2BlockTimes: ok([{ value: 2, timestamp: 0 }]),
      fetchL2GasUsed: ok([{ value: 3, timestamp: 0 }]),
      fetchSequencerDistribution: ok([{ name: 'foo', value: 1 }]),
      fetchBlockTransactions: ok([{ block: 1, txs: 2, sequencer: 'bar' }]),
      fetchBatchBlobCounts: ok([{ block: 10, batch: 1, blobs: 2 }]),
      fetchL2TxFee: ok(12),
      fetchCloudCost: ok(13),
    });

    const res = await fetchMainDashboardData('1h', null);
    expect(res.avgProve).toBe(3);
    expect(res.avgVerify).toBe(4);
    expect(res.sequencerDist[0].name).toBe('foo');
    expect(res.txPerBlock).toHaveLength(1);
    expect(res.badRequestResults).toHaveLength(18);
  });

  it('defaults to empty arrays when service data missing', async () => {
    setAll({
      fetchL2BlockCadence: ok(null),
      fetchBatchPostingCadence: ok(null),
      fetchAvgProveTime: ok(null),
      fetchAvgVerifyTime: ok(null),
      fetchAvgL2Tps: ok(null),
      fetchPreconfData: ok(null),
      fetchL2Reorgs: ok(null),
      fetchSlashingEventCount: ok(null),
      fetchForcedInclusionCount: ok(null),
      fetchL2HeadBlock: ok(null),
      fetchL1HeadBlock: ok(null),
      fetchProveTimes: ok(null),
      fetchVerifyTimes: ok(null),
      fetchL2BlockTimes: ok(null),
      fetchL2GasUsed: ok(null),
      fetchSequencerDistribution: ok(null),
      fetchBlockTransactions: ok(null),
      fetchBatchBlobCounts: ok(null),
      fetchL2TxFee: ok(null),
      fetchCloudCost: ok(null),
    });

    const res = await fetchMainDashboardData('1h', null);
    expect(res.proveTimes).toEqual([]);
    expect(res.sequencerDist).toEqual([]);
    expect(res.txPerBlock).toEqual([]);
    expect(res.blobsPerBatch).toEqual([]);
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
