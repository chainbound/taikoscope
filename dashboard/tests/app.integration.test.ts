import { it, expect } from 'vitest';
import {
  fetchAvgProveTime,
  fetchAvgVerifyTime,
  fetchL2BlockCadence,
  fetchBatchPostingCadence,
  fetchActiveGateways,
  fetchL2Reorgs,
  fetchL2ReorgEvents,
  fetchSlashingEventCount,
  fetchForcedInclusionCount,
  fetchSlashingEvents,
  fetchForcedInclusionEvents,
  fetchCurrentOperator,
  fetchNextOperator,
  fetchL2HeadBlock,
  fetchL1HeadBlock,
  fetchL2HeadNumber,
  fetchL1HeadNumber,
  fetchProveTimes,
  fetchVerifyTimes,
  fetchL1BlockTimes,
  fetchL2BlockTimes,
  fetchL2GasUsed,
  fetchSequencerDistribution,
  API_BASE,
} from '../services/apiService.ts';
import { createMetrics, hasBadRequest } from '../helpers.js';
import type { MetricData } from '../types.js';

type TimeRange = '1h' | '24h' | '7d';

type State = {
  metrics: MetricData[];
  secondsToProveData: unknown[];
  secondsToVerifyData: unknown[];
  l2BlockTimeData: unknown[];
  l2GasUsedData: unknown[];
  l1BlockTimeData: unknown[];
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
  '/l2-block-cadence?range=1h': { l2_block_cadence_ms: 60000 },
  '/batch-posting-cadence?range=1h': { batch_posting_cadence_ms: 120000 },
  '/avg-prove-time?range=1h': { avg_prove_time_ms: 1500 },
  '/avg-verify-time?range=1h': { avg_verify_time_ms: 2500 },
  '/active-gateways?range=1h': { gateways: ['gw1', 'gw2'] },
  '/current-operator': { operator: '0xaaa' },
  '/next-operator': { operator: '0xbbb' },
  '/reorgs?range=1h': { events: [{ l2_block_number: 10, depth: 1 }] },
  '/slashings?range=1h': {
    events: [{ l1_block_number: 5, validator_addr: [1, 2] }],
  },
  '/forced-inclusions?range=1h': { events: [{ blob_hash: [3, 4] }] },
  '/l2-block-times?range=1h': {
    blocks: [
      { l2_block_number: 1, ms_since_prev_block: 1000 },
      { l2_block_number: 2, ms_since_prev_block: 2000 },
    ],
  },
  '/l1-block-times?range=1h': {
    blocks: [
      { block_number: 50, minute: 1 },
      { block_number: 52, minute: 2 },
    ],
  },
  '/prove-times?range=1h': {
    batches: [{ batch_id: 1, seconds_to_prove: 3 }],
  },
  '/verify-times?range=1h': {
    batches: [{ batch_id: 1, seconds_to_verify: 4 }],
  },
  '/l2-gas-used?range=1h': {
    blocks: [
      { l2_block_number: 1, gas_used: 100 },
      { l2_block_number: 2, gas_used: 150 },
    ],
  },
  '/sequencer-distribution?range=1h': {
    sequencers: [{ address: 'addr1', blocks: 10 }],
  },
  '/l2-head-block': { l2_head_block: 123 },
  '/l1-head-block': { l1_head_block: 456 },
};

(globalThis as {
  fetch?: (url: string) => Promise<MockFetchResponse>;
}).fetch = async (url: string): Promise<MockFetchResponse> => {
  const u = new URL(url, 'http://localhost');
  const key = u.pathname + (u.search ? `?${u.searchParams.toString()}` : '');
  return {
    ok: true,
    json: async () => responses[key] ?? {},
  };
};

class MockEventSource {
  url: string;
  onmessage: ((e: { data: string }) => void) | null = null;
  onerror: (() => void) | null = null;
  closed = false;
  constructor(url: string) {
    this.url = url;
  }
  emitMessage(data: string) {
    this.onmessage?.({ data });
  }
  emitError() {
    this.onerror?.();
  }
  close() {
    this.closed = true;
  }
}
(globalThis as unknown as { EventSource: unknown }).EventSource =
  MockEventSource as unknown as EventSource;

interface IntervalId { fn: () => Promise<void> | void; ms: number }
let intervals: IntervalId[] = [];
(globalThis as unknown as {
  setInterval: (fn: () => Promise<void> | void, ms: number) => NodeJS.Timeout;
}).setInterval = (
  fn: () => Promise<void> | void,
  ms: number,
): NodeJS.Timeout => {
  const id: IntervalId = { fn, ms };
  intervals.push(id);
  return id as unknown as NodeJS.Timeout;
};
(globalThis as unknown as { clearInterval: (id: NodeJS.Timeout) => void }).clearInterval = (
  id: NodeJS.Timeout,
) => {
  intervals = intervals.filter((i) => i !== (id as unknown as IntervalId));
};

async function fetchData(range: TimeRange, state: State) {
  const [
    l2CadenceRes,
    batchCadenceRes,
    avgProveRes,
    avgVerifyRes,
    activeGatewaysRes,
    currentOperatorRes,
    nextOperatorRes,
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
    l1TimesRes,
    l2TimesRes,
    l2GasUsedRes,
    sequencerDistRes,
  ] = await Promise.all([
    fetchL2BlockCadence(range),
    fetchBatchPostingCadence(range),
    fetchAvgProveTime(range),
    fetchAvgVerifyTime(range),
    fetchActiveGateways(range),
    fetchCurrentOperator(),
    fetchNextOperator(),
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
    fetchL1BlockTimes(range),
    fetchL2BlockTimes(range),
    fetchL2GasUsed(range),
    fetchSequencerDistribution(range),
  ]);

  const l2Cadence = l2CadenceRes.data;
  const batchCadence = batchCadenceRes.data;
  const avgProve = avgProveRes.data;
  const avgVerify = avgVerifyRes.data;
  const activeGateways = activeGatewaysRes.data;
  const currentOperator = currentOperatorRes.data;
  const nextOperator = nextOperatorRes.data;
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
  const l1Times = l1TimesRes.data || [];
  const l2Times = l2TimesRes.data || [];
  const l2Gas = l2GasUsedRes.data || [];
  const sequencerDist = sequencerDistRes.data || [];

  const anyBadRequest = hasBadRequest([
    l2CadenceRes,
    batchCadenceRes,
    avgProveRes,
    avgVerifyRes,
    activeGatewaysRes,
    currentOperatorRes,
    nextOperatorRes,
    l2ReorgsRes,
    l2ReorgEventsRes,
    slashingCountRes,
    forcedInclusionCountRes,
    l2BlockRes,
    l1BlockRes,
    proveTimesRes,
    verifyTimesRes,
    l1TimesRes,
    l2TimesRes,
    l2GasUsedRes,
    sequencerDistRes,
  ]);

  const currentMetrics: MetricData[] = createMetrics({
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
    l2Block,
    l1Block,
  });

  state.metrics = currentMetrics;
  state.secondsToProveData = proveTimes;
  state.secondsToVerifyData = verifyTimes;
  state.l2BlockTimeData = l2Times;
  state.l2GasUsedData = l2Gas;
  state.l1BlockTimeData = l1Times;
  state.sequencerDistribution = sequencerDist;
  state.slashingEvents = slashingEventsData;
  state.forcedInclusionEvents = forcedInclusionEventsData;
  state.l2ReorgEvents = reorgEvents;
  state.l2HeadBlock =
    currentMetrics.find((m) => m.title === 'L2 Head Block')?.value || 'N/A';
  state.l1HeadBlock =
    currentMetrics.find((m) => m.title === 'L1 Head Block')?.value || 'N/A';
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
      m.title === 'L1 Head Block' ? { ...m, value } : m,
    );
  }
  if (l2.data !== null) {
    const value = l2.data.toLocaleString();
    state.l2HeadBlock = value;
    state.metrics = state.metrics.map((m) =>
      m.title === 'L2 Head Block' ? { ...m, value } : m,
    );
  }
}

function setupSSE(state: State) {
  let pollId: ReturnType<typeof setInterval> | null = null;
  const startPolling = () => {
    if (!pollId) {
      state.errorMessage =
        'Realtime updates unavailable, falling back to polling.';
      updateHeads(state);
      pollId = setInterval(() => updateHeads(state), 10000);
    }
  };

  const l1Source = new EventSource(`${API_BASE}/sse/l1-head`);
  const l2Source = new EventSource(`${API_BASE}/sse/l2-head`);

  l1Source.onmessage = (e) => {
    const value = Number(e.data).toLocaleString();
    state.l1HeadBlock = value;
    state.metrics = state.metrics.map((m) =>
      m.title === 'L1 Head Block' ? { ...m, value } : m,
    );
  };
  l2Source.onmessage = (e) => {
    const value = Number(e.data).toLocaleString();
    state.l2HeadBlock = value;
    state.metrics = state.metrics.map((m) =>
      m.title === 'L2 Head Block' ? { ...m, value } : m,
    );
  };

  const handleError = () => {
    l1Source.close();
    l2Source.close();
    startPolling();
  };

  l1Source.onerror = handleError;
  l2Source.onerror = handleError;

  return { l1Source, l2Source };
}

it('app integration', async () => {
  const state: State = {
    metrics: [],
    secondsToProveData: [],
    secondsToVerifyData: [],
    l2BlockTimeData: [],
    l2GasUsedData: [],
    l1BlockTimeData: [],
    sequencerDistribution: [],
    l2ReorgEvents: [],
    slashingEvents: [],
    forcedInclusionEvents: [],
    l2HeadBlock: '0',
    l1HeadBlock: '0',
    errorMessage: '',
  };

  await fetchData('1h', state);
  expect(state.metrics.length > 0).toBe(true);
  expect(state.secondsToProveData.length).toBe(1);
  expect(state.l2GasUsedData.length).toBe(1);

  const { l1Source } = setupSSE(state);
  (l1Source as unknown as MockEventSource).emitError();
  expect(state.errorMessage.includes('falling back to polling')).toBe(true);
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
    'Operators',
    'Other',
  ];
  const visible = groupOrder.filter((g) => grouped[g] && grouped[g].length > 0);
  const expected = ['Network Performance', 'Network Health', 'Operators'];
  expect(visible).toStrictEqual(expected);

  console.log('App integration tests passed.');
});
