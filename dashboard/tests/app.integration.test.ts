import { it, expect } from 'vitest';
import {
  fetchAvgProveTime,
  fetchAvgVerifyTime,
  fetchL2BlockCadence,
  fetchBatchPostingCadence,
  fetchPreconfData,
  fetchL2Reorgs,
  fetchL2ReorgEvents,
  fetchSlashingEventCount,
  fetchForcedInclusionCount,
  fetchSlashingEvents,
  fetchForcedInclusionEvents,
  fetchL2HeadBlock,
  fetchL1HeadBlock,
  fetchL2HeadNumber,
  fetchL1HeadNumber,
  fetchProveTimes,
  fetchVerifyTimes,
  fetchL2BlockTimes,
  fetchL2GasUsed,
  fetchSequencerDistribution,
  fetchL2Fees,
  fetchCloudCost,
} from '../services/apiService.ts';
import { createMetrics, hasBadRequest } from '../helpers';
import type { MetricData } from '../types';

type TimeRange = string;

type State = {
  metrics: MetricData[];
  secondsToProveData: unknown[];
  secondsToVerifyData: unknown[];
  l2BlockTimeData: unknown[];
  l2GasUsedData: unknown[];
  sequencerDistribution: unknown[];
  l2ReorgEvents: unknown[];
  slashingEvents: unknown[];
  forcedInclusionEvents: unknown[];
  l2HeadBlock: string;
  l1HeadBlock: string;
  errorMessage: string;
};

interface MockFetchResponse {
  ok: boolean;
  json: () => Promise<unknown>;
}

const responses: Record<string, Record<string, unknown>> = {
  '/v1/l2-block-cadence?range=15m': { l2_block_cadence_ms: 60000 },
  '/v1/batch-posting-cadence?range=15m': { batch_posting_cadence_ms: 120000 },
  '/v1/avg-prove-time?range=15m': { avg_prove_time_ms: 1500 },
  '/v1/avg-verify-time?range=15m': { avg_verify_time_ms: 2500 },
  '/v1/l2-block-cadence?range=1h': { l2_block_cadence_ms: 60000 },
  '/v1/batch-posting-cadence?range=1h': { batch_posting_cadence_ms: 120000 },
  '/v1/avg-prove-time?range=1h': { avg_prove_time_ms: 1500 },
  '/v1/avg-verify-time?range=1h': { avg_verify_time_ms: 2500 },
  '/v1/dashboard-data?range=1h': {
    preconf_data: {
      candidates: ['gw1', 'gw2'],
      current_operator: '0xaaa',
      next_operator: '0xbbb',
    },
  },
  '/v1/reorgs?range=1h': {
    events: [
      { l2_block_number: 10, depth: 1, inserted_at: '1970-01-01T00:00:00Z' },
    ],
  },
  '/v1/slashings?range=1h': {
    events: [{ l1_block_number: 5, validator_addr: [1, 2] }],
  },
  '/v1/forced-inclusions?range=1h': { events: [{ blob_hash: [3, 4] }] },
  '/v1/reorgs?range=15m': {
    events: [
      { l2_block_number: 10, depth: 1, inserted_at: '1970-01-01T00:00:00Z' },
    ],
  },
  '/v1/slashings?range=15m': {
    events: [{ l1_block_number: 5, validator_addr: [1, 2] }],
  },
  '/v1/forced-inclusions?range=15m': { events: [{ blob_hash: [3, 4] }] },
  '/v1/l2-block-times?range=1h': {
    blocks: [
      { l2_block_number: 1, ms_since_prev_block: 1000 },
      { l2_block_number: 2, ms_since_prev_block: 2000 },
    ],
  },
  '/v1/l2-block-times?range=15m': {
    blocks: [
      { l2_block_number: 1, ms_since_prev_block: 1000 },
      { l2_block_number: 2, ms_since_prev_block: 2000 },
    ],
  },
  '/v1/l1-block-times?range=1h': {
    blocks: [
      { block_number: 50, minute: 1 },
      { block_number: 52, minute: 2 },
    ],
  },
  '/v1/l1-block-times?range=15m': {
    blocks: [
      { block_number: 50, minute: 1 },
      { block_number: 52, minute: 2 },
    ],
  },
  '/v1/prove-times?range=1h': {
    batches: [{ batch_id: 1, seconds_to_prove: 3 }],
  },
  '/v1/prove-times?range=15m': {
    batches: [{ batch_id: 1, seconds_to_prove: 3 }],
  },
  '/v1/verify-times?range=1h': {
    batches: [{ batch_id: 1, seconds_to_verify: 4 }],
  },
  '/v1/verify-times?range=15m': {
    batches: [{ batch_id: 1, seconds_to_verify: 4 }],
  },
  '/v1/l2-gas-used?range=1h': {
    blocks: [
      { l2_block_number: 1, gas_used: 100 },
      { l2_block_number: 2, gas_used: 150 },
    ],
  },
  '/v1/l2-gas-used?range=15m': {
    blocks: [
      { l2_block_number: 1, gas_used: 100 },
      { l2_block_number: 2, gas_used: 150 },
    ],
  },
  '/v1/l2-fees?range=1h': { priority_fee: 600, base_fee: 400 },
  '/v1/l2-fees?range=15m': { priority_fee: 600, base_fee: 400 },
  '/v1/cloud-cost?range=1h': { cost_usd: 3 },
  '/v1/cloud-cost?range=15m': { cost_usd: 3 },
  '/v1/sequencer-distribution?range=1h': {
    sequencers: [{ address: 'addr1', blocks: 10 }],
  },
  '/v1/sequencer-distribution?range=15m': {
    sequencers: [{ address: 'addr1', blocks: 10 }],
  },
  '/v1/l2-head-block': { l2_head_block: 123 },
  '/v1/l1-head-block': { l1_head_block: 456 },
  '/v1/l2-fees?range=24h': { priority_fee: 1200, base_fee: 800 },
  '/v1/cloud-cost?range=24h': { cost_usd: 72 },
};

(
  globalThis as {
    fetch?: (url: string) => Promise<MockFetchResponse>;
  }
).fetch = async (url: string): Promise<MockFetchResponse> => {
  const u = new URL(url, 'http://localhost');
  const key = u.pathname + (u.search ? `?${u.searchParams.toString()}` : '');
  return {
    ok: true,
    json: async () => responses[key] ?? {},
  };
};

interface IntervalId {
  fn: () => Promise<void> | void;
  ms: number;
}
let intervals: IntervalId[] = [];
(
  globalThis as unknown as {
    setInterval: (fn: () => Promise<void> | void, ms: number) => NodeJS.Timeout;
  }
).setInterval = (
  fn: () => Promise<void> | void,
  ms: number,
): NodeJS.Timeout => {
  const id: IntervalId = { fn, ms };
  intervals.push(id);
  return id as unknown as NodeJS.Timeout;
};
(
  globalThis as unknown as { clearInterval: (id: NodeJS.Timeout) => void }
).clearInterval = (id: NodeJS.Timeout) => {
  intervals = intervals.filter((i) => i !== (id as unknown as IntervalId));
};

async function fetchData(range: TimeRange, state: State, economics = false) {
  if (economics) {
    const [l2FeesRes, l2BlockRes, l1BlockRes] = await Promise.all([
      fetchL2Fees(range, undefined),
      fetchL2HeadBlock(range),
      fetchL1HeadBlock(range),
    ]);

    const l2FeeData = l2FeesRes.data;
    const priorityFee = l2FeeData?.priority_fee ?? null;
    const baseFee = l2FeeData?.base_fee ?? null;
    const l2Block = l2BlockRes.data;
    const l1Block = l1BlockRes.data;

    const anyBadRequest = hasBadRequest([l2FeesRes, l2BlockRes, l1BlockRes]);

    const currentMetrics: MetricData[] = createMetrics({
      avgTps: 3,
      l2Cadence: null,
      batchCadence: null,
      avgProve: null,
      avgVerify: null,
      activeGateways: null,
      currentOperator: null,
      nextOperator: null,
      l2Reorgs: null,
      slashings: null,
      forcedInclusions: null,
      priorityFee,
      baseFee,
      l2Block,
      l1Block,
      cloudCost: null,
    });

    state.metrics = currentMetrics;
    state.l2HeadBlock =
      currentMetrics.find((m) => m.title === 'L2 Head Block')?.value || 'N/A';
    state.l1HeadBlock =
      currentMetrics.find((m) => m.title === 'L1 Head Block')?.value || 'N/A';
    state.errorMessage = anyBadRequest
      ? 'Invalid parameters provided. Some data may not be available.'
      : '';
    return;
  }
  const [
    l2CadenceRes,
    batchCadenceRes,
    avgProveRes,
    avgVerifyRes,
    preconfRes,
    l2ReorgsRes,
    l2ReorgEventsRes,
    slashingCountRes,
    forcedInclusionCountRes,
    slashingEventsRes,
    forcedInclusionEventsRes,
    l2BlockRes,
    l1BlockRes,
    proveTimesRes,
    verifyTimesRes,
    l2TimesRes,
    l2GasUsedRes,
    sequencerDistRes,
    l2FeesRes,
    cloudCostRes,
  ] = await Promise.all([
    fetchL2BlockCadence(range, undefined),
    fetchBatchPostingCadence(range),
    fetchAvgProveTime(range),
    fetchAvgVerifyTime(range),
    fetchPreconfData(),
    fetchL2Reorgs(range),
    fetchL2ReorgEvents(range),
    fetchSlashingEventCount(range),
    fetchForcedInclusionCount(range),
    fetchSlashingEvents(range),
    fetchForcedInclusionEvents(range),
    fetchL2HeadBlock(range),
    fetchL1HeadBlock(range),
    fetchProveTimes(range),
    fetchVerifyTimes(range),
    fetchL2BlockTimes(range, undefined),
    fetchL2GasUsed(range, undefined),
    fetchSequencerDistribution(range),
    fetchL2Fees(range, undefined),
    fetchCloudCost(range),
  ]);

  const l2Cadence = l2CadenceRes.data;
  const batchCadence = batchCadenceRes.data;
  const avgProve = avgProveRes.data;
  const avgVerify = avgVerifyRes.data;
  const preconfData = preconfRes.data;
  const activeGateways = preconfData ? preconfData.candidates.length : null;
  const currentOperator = preconfData?.current_operator ?? null;
  const nextOperator = preconfData?.next_operator ?? null;
  const l2Reorgs = l2ReorgsRes.data;
  const reorgEvents = l2ReorgEventsRes.data || [];
  const slashings = slashingCountRes.data;
  const forcedInclusions = forcedInclusionCountRes.data;
  const slashingEventsData = slashingEventsRes.data || [];
  const forcedInclusionEventsData = forcedInclusionEventsRes.data || [];
  const l2Block = l2BlockRes.data;
  const l1Block = l1BlockRes.data;
  const proveTimes = proveTimesRes.data || [];
  const verifyTimes = verifyTimesRes.data || [];
  const l2Times = l2TimesRes.data || [];
  const l2Gas = l2GasUsedRes.data || [];
  const sequencerDist = sequencerDistRes.data || [];
  const l2FeeData = l2FeesRes.data;
  const priorityFee = l2FeeData?.priority_fee ?? null;
  const baseFee = l2FeeData?.base_fee ?? null;
  const cloudCost = cloudCostRes.data;

  const anyBadRequest = hasBadRequest([
    l2CadenceRes,
    batchCadenceRes,
    avgProveRes,
    avgVerifyRes,
    preconfRes,
    l2ReorgsRes,
    l2ReorgEventsRes,
    slashingCountRes,
    forcedInclusionCountRes,
    l2BlockRes,
    l1BlockRes,
    proveTimesRes,
    verifyTimesRes,
    l2TimesRes,
    l2GasUsedRes,
    sequencerDistRes,
    l2FeesRes,
  ]);

  const currentMetrics: MetricData[] = createMetrics({
    avgTps: 3,
    l2Cadence,
    batchCadence,
    avgProve,
    avgVerify,
    activeGateways,
    currentOperator,
    nextOperator,
    l2Reorgs,
    slashings,
    forcedInclusions,
    priorityFee,
    baseFee,
    cloudCost,
    l2Block,
    l1Block,
  });

  state.metrics = currentMetrics;
  state.secondsToProveData = proveTimes;
  state.secondsToVerifyData = verifyTimes;
  state.l2BlockTimeData = l2Times;
  state.l2GasUsedData = l2Gas;
  state.sequencerDistribution = sequencerDist;
  state.slashingEvents = slashingEventsData;
  state.forcedInclusionEvents = forcedInclusionEventsData;
  state.l2ReorgEvents = reorgEvents;
  state.l2HeadBlock =
    currentMetrics.find((m) => m.title === 'L2 Block')?.value || 'N/A';
  state.l1HeadBlock =
    currentMetrics.find((m) => m.title === 'L1 Block')?.value || 'N/A';
  if (anyBadRequest) {
    state.errorMessage =
      'Invalid parameters provided. Some data may not be available.';
  } else {
    state.errorMessage = '';
  }
}

async function updateHeads(state: State) {
  const [l1, l2] = await Promise.all([
    fetchL1HeadNumber(),
    fetchL2HeadNumber(),
  ]);
  if (l1.data !== null) {
    const value = l1.data.toLocaleString();
    state.l1HeadBlock = value;
    state.metrics = state.metrics.map((m) =>
      m.title === 'L1 Block' ? { ...m, value } : m,
    );
  }
  if (l2.data !== null) {
    const value = l2.data.toLocaleString();
    state.l2HeadBlock = value;
    state.metrics = state.metrics.map((m) =>
      m.title === 'L2 Block' ? { ...m, value } : m,
    );
  }
}

function setupPolling(state: State) {
  updateHeads(state);
  return setInterval(() => updateHeads(state), 60000);
}

it('app integration', async () => {
  const state: State = {
    metrics: [],
    secondsToProveData: [],
    secondsToVerifyData: [],
    l2BlockTimeData: [],
    l2GasUsedData: [],
    sequencerDistribution: [],
    l2ReorgEvents: [],
    slashingEvents: [],
    forcedInclusionEvents: [],
    l2HeadBlock: '0',
    l1HeadBlock: '0',
    errorMessage: '',
  };

  await fetchData('15m', state);
  expect(state.metrics.length > 0).toBe(true);

  await fetchData('1h', state);
  expect(state.metrics.length > 0).toBe(true);
  expect(state.secondsToProveData.length).toBe(1);
  expect(state.l2GasUsedData.length).toBe(2);

  await fetchData('15m', state, true);
  await fetchData('1h', state, true);
  const econMetric = state.metrics.find((m) => m.group === 'Network Economics');
  expect(econMetric).toBeDefined();

  setupPolling(state);
  expect(intervals.length).toBe(1);
  await intervals[0].fn();
  expect(state.l1HeadBlock).toBe('456');
  expect(state.l2HeadBlock).toBe('123');

  const grouped = state.metrics.reduce<Record<string, MetricData[]>>(
    (acc, m) => {
      const g = m.group ?? 'Other';
      if (!acc[g]) acc[g] = [];
      acc[g].push(m);
      return acc;
    },
    {},
  );
  const groupOrder = [
    'Network Performance',
    'Network Health',
    'Sequencers',
    'Other',
  ];
  const visible = groupOrder.filter((g) => grouped[g] && grouped[g].length > 0);
  const expected = ['Network Performance', 'Network Health', 'Sequencers'];
  expect(visible).toStrictEqual(expected);

  console.log('App integration tests passed.');
});
