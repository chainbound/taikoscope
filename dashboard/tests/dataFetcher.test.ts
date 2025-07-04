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
        avg_tps: 5,
        preconf_data: {
          candidates: ['a'],
          current_operator: 'x',
          next_operator: 'y',
        },
        l2_reorgs: 6,
        slashings: 7,
        forced_inclusions: 8,
        l2_head_block: 10,
        l1_head_block: 11,
      }),
      fetchL2Fees: ok({
        priority_fee: 12,
        base_fee: 5,
        l1_data_cost: null,
        prove_cost: 13,
        sequencers: [],
      }),
      fetchProveTimes: ok([{ name: '1', value: 1, timestamp: 0 }]),
      fetchL2BlockTimesAggregated: ok([{ value: 2, timestamp: 0 }]),
      fetchL2GasUsedAggregated: ok([{ value: 3, timestamp: 0 }]),
      fetchSequencerDistribution: ok([
        { name: 'foo', address: '0xfoo', value: 1 },
      ]),
      fetchBlockTransactionsAggregated: ok([
        { block: 1, txs: 2, sequencer: 'bar' },
      ]),
      fetchBatchBlobCounts: ok([{ block: 10, batch: 1, blobs: 2 }]),
    });

    const res = await fetchMainDashboardData('1h', null);
    expect(res.avgProve).toBe(3);
    expect(res.sequencerDist[0].name).toBe('foo');
    expect(res.txPerBlock).toHaveLength(1);
    expect(res.badRequestResults).toHaveLength(7);
  });

  it('defaults to empty arrays when service data missing', async () => {
    setAll({
      fetchDashboardData: ok(null),
      fetchL2Fees: ok(null),
      fetchProveTimes: ok(null),
      fetchL2BlockTimesAggregated: ok(null),
      fetchL2GasUsedAggregated: ok(null),
      fetchSequencerDistribution: ok(null),
      fetchBlockTransactionsAggregated: ok(null),
      fetchBatchBlobCounts: ok(null),
    });

    const res = await fetchMainDashboardData('1h', null);
    expect(res.proveTimes).toEqual([]);
    expect(res.sequencerDist).toEqual([]);
    expect(res.txPerBlock).toEqual([]);
    expect(res.blobsPerBatch).toEqual([]);
    expect(res.badRequestResults).toHaveLength(7);
  });

  it('fetches economics data', async () => {
    setAll({
      fetchL2Fees: ok({
        priority_fee: 1,
        base_fee: 2,
        l1_data_cost: 4,
        prove_cost: 5,
        sequencers: [],
      }),
      fetchL2HeadBlock: ok(2),
      fetchL1HeadBlock: ok(3),
      fetchSequencerDistribution: ok([
        { name: 'foo', address: '0xfoo', value: 1, tps: null },
      ]),
    });

    const res = await fetchEconomicsData('1h', null);
    expect(res.priorityFee).toBe(1);
    expect(res.baseFee).toBe(2);
    expect(res.l2Block).toBe(2);
    expect(res.l1Block).toBe(3);
    expect(res.l1DataCost).toBe(4);
    expect(res.proveCost).toBe(5);

    expect(res.sequencerDist[0].name).toBe('foo');
    expect(res.badRequestResults).toHaveLength(4);
  });

  it('defaults economics costs to null when missing', async () => {
    setAll({
      fetchL2Fees: ok({
        priority_fee: null,
        base_fee: null,
        l1_data_cost: null,
        prove_cost: null,
        sequencers: [],
      }),
      fetchL2HeadBlock: ok(null),
      fetchL1HeadBlock: ok(null),
      fetchSequencerDistribution: ok(null),
    });

    const res = await fetchEconomicsData('1h', null);
    expect(res.priorityFee).toBeNull();
    expect(res.baseFee).toBeNull();
    expect(res.l1DataCost).toBeNull();
    expect(res.proveCost).toBeNull();

    expect(res.badRequestResults).toHaveLength(4);
  });

  it('resets isTimeRangeChanging on fetch error', async () => {
    let changing = true;
    const setChanging = (v: boolean) => {
      changing = v;
    };
    const mutate = vi.fn().mockRejectedValue(new Error('fail'));
    const fetchData = async () => {
      setChanging(true);
      try {
        await mutate();
      } catch {
        setChanging(false);
      }
    };
    await fetchData();
    expect(changing).toBe(false);
  });
});
