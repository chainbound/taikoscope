import React, { useState, useEffect, useCallback, lazy } from 'react';
import { createMetrics, hasBadRequest } from './helpers';
import { DashboardHeader } from './components/DashboardHeader';
import { MetricCard } from './components/MetricCard';
import { MetricCardSkeleton } from './components/MetricCardSkeleton';
import { ChartCard } from './components/ChartCard';
import { DataTable } from './components/DataTable';
const SequencerPieChart = lazy(() =>
  import('./components/SequencerPieChart').then((m) => ({
    default: m.SequencerPieChart,
  })),
);
const BlockTimeChart = lazy(() =>
  import('./components/BlockTimeChart').then((m) => ({
    default: m.BlockTimeChart,
  })),
);
const BatchProcessChart = lazy(() =>
  import('./components/BatchProcessChart').then((m) => ({
    default: m.BatchProcessChart,
  })),
);
const GasUsedChart = lazy(() =>
  import('./components/GasUsedChart').then((m) => ({
    default: m.GasUsedChart,
  })),
);
const ReorgDepthChart = lazy(() =>
  import('./components/ReorgDepthChart').then((m) => ({
    default: m.ReorgDepthChart,
  })),
);
const BlockTxChart = lazy(() =>
  import('./components/BlockTxChart').then((m) => ({
    default: m.BlockTxChart,
  })),
);
const BlobsPerBatchChart = lazy(() =>
  import('./components/BlobsPerBatchChart').then((m) => ({
    default: m.BlobsPerBatchChart,
  })),
);
const TpsChart = lazy(() =>
  import('./components/TpsChart').then((m) => ({
    default: m.TpsChart,
  })),
);
import {
  TimeRange,
  TimeSeriesData,
  PieChartDataItem,
  MetricData,
  L2ReorgEvent,
  SlashingEvent,
  ForcedInclusionEvent,
} from './types';
import { bytesToHex, loadRefreshRate, saveRefreshRate } from './utils';
import { getSequencerAddress, getSequencerName } from './sequencerConfig.js';
import {
  API_BASE,
  fetchAvgProveTime,
  fetchAvgVerifyTime,
  fetchL2BlockCadence,
  fetchBatchPostingCadence,
  fetchActiveGateways,
  fetchActiveGatewayAddresses,
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
  fetchSequencerBlocks,
  fetchBlockTransactions,
  fetchBatchBlobCounts,
  fetchAvgL2Tps,
  type BlockTransaction,
  type BatchBlobCount,
} from './services/apiService';

// Updated Taiko Pink
const TAIKO_PINK = '#e81899';

const App: React.FC = () => {
  const [timeRange, setTimeRange] = useState<TimeRange>('1h');
  const [selectedSequencer, setSelectedSequencer] = useState<string | null>(
    null,
  );
  const [metrics, setMetrics] = useState<MetricData[]>([]);
  const [loadingMetrics, setLoadingMetrics] = useState(true);
  const [secondsToProveData, setSecondsToProveData] = useState<
    TimeSeriesData[]
  >([]);
  const [secondsToVerifyData, setSecondsToVerifyData] = useState<
    TimeSeriesData[]
  >([]);
  const [l2BlockTimeData, setL2BlockTimeData] = useState<TimeSeriesData[]>([]);
  const [l2GasUsedData, setL2GasUsedData] = useState<TimeSeriesData[]>([]);
  const [l1BlockTimeData, setL1BlockTimeData] = useState<TimeSeriesData[]>([]);
  const [blockTxData, setBlockTxData] = useState<BlockTransaction[]>([]);
  const [batchBlobCounts, setBatchBlobCounts] = useState<BatchBlobCount[]>([]);
  const [sequencerDistribution, setSequencerDistribution] = useState<
    PieChartDataItem[]
  >([]);
  const sequencerList = React.useMemo(
    () => sequencerDistribution.map((s) => s.name),
    [sequencerDistribution],
  );
  const [l2HeadBlock, setL2HeadBlock] = useState<string>('0');
  const [l1HeadBlock, setL1HeadBlock] = useState<string>('0');
  const [refreshRate, setRefreshRate] = useState<number>(() =>
    loadRefreshRate(),
  );
  const [lastRefresh, setLastRefresh] = useState<number>(Date.now());
  const [errorMessage, setErrorMessage] = useState<string>('');
  const [seqDistTxPage, setSeqDistTxPage] = useState<number>(0);
  const [tableLoading, setTableLoading] = useState<boolean>(
    new URLSearchParams(window.location.search).get('view') === 'table',
  );
  const [tableView, setTableView] = useState<null | {
    title: string;
    columns: { key: string; label: string }[];
    rows: Record<string, string | number>[];
    onRowClick?: (row: Record<string, string | number>) => void;
    extraAction?: { label: string; onClick: () => void };
    extraTable?: {
      title: string;
      columns: { key: string; label: string }[];
      rows: Record<string, string | number>[];
      onRowClick?: (row: Record<string, string | number>) => void;
      pagination?: {
        page: number;
        onNext: () => void;
        onPrev: () => void;
        disableNext?: boolean;
        disablePrev?: boolean;
      };
    };
    timeRange?: TimeRange;
    onTimeRangeChange?: (range: TimeRange) => void;
    chart?: React.ReactNode;
  }>(null);

  useEffect(() => {
    let pollId: NodeJS.Timeout | null = null;

    const updateHeads = async () => {
      const [l1, l2] = await Promise.all([
        fetchL1HeadNumber(),
        fetchL2HeadNumber(),
      ]);
      if (l1.data !== null) {
        const value = l1.data.toLocaleString();
        setL1HeadBlock(value);
        setMetrics((m) =>
          m.map((metric) =>
            metric.title === 'L1 Head Block' ? { ...metric, value } : metric,
          ),
        );
      }
      if (l2.data !== null) {
        const value = l2.data.toLocaleString();
        setL2HeadBlock(value);
        setMetrics((m) =>
          m.map((metric) =>
            metric.title === 'L2 Head Block' ? { ...metric, value } : metric,
          ),
        );
      }
    };

    const startPolling = () => {
      if (!pollId) {
        setErrorMessage(
          'Realtime updates unavailable, falling back to polling.',
        );
        updateHeads();
        pollId = setInterval(updateHeads, 60000);
      }
    };

    const l1Source = new EventSource(`${API_BASE}/sse/l1-head`);
    const l2Source = new EventSource(`${API_BASE}/sse/l2-head`);

    l1Source.onmessage = (e) => {
      const value = Number(e.data).toLocaleString();
      setL1HeadBlock(value);
      setMetrics((m) =>
        m.map((metric) =>
          metric.title === 'L1 Head Block' ? { ...metric, value } : metric,
        ),
      );
    };
    l2Source.onmessage = (e) => {
      const value = Number(e.data).toLocaleString();
      setL2HeadBlock(value);
      setMetrics((m) =>
        m.map((metric) =>
          metric.title === 'L2 Head Block' ? { ...metric, value } : metric,
        ),
      );
    };

    const handleError = () => {
      l1Source.close();
      l2Source.close();
      startPolling();
    };

    l1Source.onerror = handleError;
    l2Source.onerror = handleError;

    return () => {
      l1Source.close();
      l2Source.close();
      if (pollId) clearInterval(pollId);
    };
  }, []);

  const fetchData = useCallback(async () => {
    setLoadingMetrics(true);
    setLastRefresh(Date.now());
    const range = timeRange;
    const [
      l2CadenceRes,
      batchCadenceRes,
      avgProveRes,
      avgVerifyRes,
      activeGatewaysRes,
      currentOperatorRes,
      nextOperatorRes,
      l2ReorgsRes,
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
      blockTxRes,
      batchBlobCountsRes,
      avgTpsRes,
    ] = await Promise.all([
      fetchL2BlockCadence(
        range,
        selectedSequencer ? getSequencerAddress(selectedSequencer) : undefined,
      ),
      fetchBatchPostingCadence(range),
      fetchAvgProveTime(range),
      fetchAvgVerifyTime(range),
      fetchActiveGateways(range),
      fetchCurrentOperator(),
      fetchNextOperator(),
      fetchL2Reorgs(range),
      fetchSlashingEventCount(range),
      fetchForcedInclusionCount(range),
      fetchL2HeadBlock(range),
      fetchL1HeadBlock(range),
      fetchProveTimes(range),
      fetchVerifyTimes(range),
      fetchL1BlockTimes(range),
      fetchL2BlockTimes(
        range,
        selectedSequencer ? getSequencerAddress(selectedSequencer) : undefined,
      ),
      fetchL2GasUsed(
        range,
        selectedSequencer ? getSequencerAddress(selectedSequencer) : undefined,
      ),
      fetchSequencerDistribution(range),
      fetchBlockTransactions(
        range,
        50,
        undefined,
        undefined,
        selectedSequencer ? getSequencerAddress(selectedSequencer) : undefined,
      ),
      fetchBatchBlobCounts(range),
      fetchAvgL2Tps(
        range,
        selectedSequencer ? getSequencerAddress(selectedSequencer) : undefined,
      ),
    ]);

    const l2Cadence = l2CadenceRes.data;
    const batchCadence = batchCadenceRes.data;
    const avgProve = avgProveRes.data;
    const avgVerify = avgVerifyRes.data;
    const activeGateways = activeGatewaysRes.data;
    const currentOperator = currentOperatorRes.data;
    const nextOperator = nextOperatorRes.data;
    const l2Reorgs = l2ReorgsRes.data;
    const slashings = slashingCountRes.data;
    const forcedInclusions = forcedInclusionCountRes.data;
    const l2Block = l2BlockRes.data;
    const l1Block = l1BlockRes.data;
    const proveTimes = proveTimesRes.data || [];
    const verifyTimes = verifyTimesRes.data || [];
    const l1Times = l1TimesRes.data || [];
    const l2Times = l2TimesRes.data || [];
    const l2Gas = l2GasUsedRes.data || [];
    const sequencerDist = sequencerDistRes.data || [];
    const txPerBlock = blockTxRes.data || [];
    const blobsPerBatch = batchBlobCountsRes.data || [];
    const avgTps = avgTpsRes.data;

    const anyBadRequest = hasBadRequest([
      l2CadenceRes,
      batchCadenceRes,
      avgProveRes,
      avgVerifyRes,
      activeGatewaysRes,
      currentOperatorRes,
      nextOperatorRes,
      l2ReorgsRes,
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
      blockTxRes,
      batchBlobCountsRes,
      avgTpsRes,
    ]);

    const currentMetrics: MetricData[] = createMetrics({
      avgTps,
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

    setMetrics(currentMetrics);
    setSecondsToProveData(proveTimes);
    setSecondsToVerifyData(verifyTimes);
    setL2BlockTimeData(l2Times);
    setL2GasUsedData(l2Gas);
    setL1BlockTimeData(l1Times);
    setBlockTxData(txPerBlock);
    setBatchBlobCounts(blobsPerBatch);
    setSequencerDistribution(sequencerDist);
    setL2HeadBlock(
      currentMetrics.find((m) => m.title === 'L2 Head Block')?.value || 'N/A',
    );
    setL1HeadBlock(
      currentMetrics.find((m) => m.title === 'L1 Head Block')?.value || 'N/A',
    );
    if (anyBadRequest) {
      setErrorMessage(
        'Invalid parameters provided. Some data may not be available.',
      );
    } else {
      setErrorMessage('');
    }
    setLoadingMetrics(false);
  }, [timeRange, selectedSequencer]);

  const handleManualRefresh = useCallback(() => {
    void fetchData();
  }, [fetchData]);

  useEffect(() => {
    saveRefreshRate(refreshRate);
  }, [refreshRate]);

  useEffect(() => {
    if (tableView) return;
    fetchData();
    const interval = setInterval(fetchData, Math.max(refreshRate, 60000));
    return () => clearInterval(interval);
  }, [timeRange, fetchData, refreshRate, tableView]);

  const groupedMetrics = metrics.reduce<Record<string, MetricData[]>>(
    (acc, m) => {
      const group = m.group ?? 'Other';
      if (!acc[group]) acc[group] = [];
      acc[group].push(m);
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
  const skeletonGroupCounts: Record<string, number> = {
    'Network Performance': 5,
    'Network Health': 3,
    Operators: 3,
  };

  const setTableUrl = (
    name: string,
    params: Record<string, string | number | undefined> = {},
  ) => {
    const url = new URL(window.location.href);
    url.searchParams.set('view', 'table');
    url.searchParams.set('table', name);
    Object.entries(params).forEach(([k, v]) => {
      if (v !== undefined) url.searchParams.set(k, String(v));
    });
    window.history.replaceState(null, '', url);
  };

  const clearTableUrl = () => {
    const url = new URL(window.location.href);
    url.searchParams.delete('view');
    url.searchParams.delete('table');
    url.searchParams.delete('address');
    url.searchParams.delete('page');
    url.searchParams.delete('start');
    url.searchParams.delete('end');
    url.searchParams.delete('range');
    window.history.replaceState(null, '', url);
  };

  const openTable = (
    title: string,
    columns: { key: string; label: string }[],
    rows: Record<string, string | number>[],
    onRowClick?: (row: Record<string, string | number>) => void,
    extraAction?: { label: string; onClick: () => void },
    extraTable?: {
      title: string;
      columns: { key: string; label: string }[];
      rows: Record<string, string | number>[];
      onRowClick?: (row: Record<string, string | number>) => void;
      pagination?: {
        page: number;
        onNext: () => void;
        onPrev: () => void;
        disableNext?: boolean;
        disablePrev?: boolean;
      };
    },
    range?: TimeRange,
    onRangeChange?: (range: TimeRange) => void,
    chart?: React.ReactNode,
  ) => {
    setTableView({
      title,
      columns,
      rows,
      onRowClick,
      extraAction,
      extraTable,
      timeRange: range,
      onTimeRangeChange: onRangeChange,
      chart,
    });
    setTableLoading(false);
  };

  const openSequencerBlocks = async (
    address: string,
    range: TimeRange = timeRange,
  ) => {
    setTableLoading(true);
    setTimeRange(range);
    const name = getSequencerName(address);
    const blocksRes = await fetchSequencerBlocks(range, address);
    setTableUrl('sequencer-blocks', { address, range });
    openTable(
      `Blocks proposed by ${name}`,
      [{ key: 'block', label: 'Block Number' }],
      (blocksRes.data || []).map((b) => ({ block: b })),
      undefined,
      undefined,
      undefined,
      range,
      (r) => openSequencerBlocks(address, r),
    );
  };

  const openL2ReorgsTable = async (range: TimeRange = timeRange) => {
    setTableLoading(true);
    setTimeRange(range);
    const eventsRes = await fetchL2ReorgEvents(range);
    const events = (eventsRes.data || []) as L2ReorgEvent[];
    setTableUrl('reorgs', { range });
    openTable(
      'L2 Reorgs',
      [
        { key: 'l2_block_number', label: 'Block Number' },
        { key: 'depth', label: 'Depth' },
      ],
      events as unknown as Record<string, string | number>[],
      undefined,
      undefined,
      undefined,
      range,
      (r) => openL2ReorgsTable(r),
      <ReorgDepthChart data={events} />,
    );
  };

  const openSlashingEventsTable = async (range: TimeRange = timeRange) => {
    setTableLoading(true);
    setTimeRange(range);
    const eventsRes = await fetchSlashingEvents(range);
    const events = (eventsRes.data || []) as SlashingEvent[];
    setTableUrl('slashings', { range });
    openTable(
      'Slashing Events',
      [
        { key: 'l1_block_number', label: 'L1 Block' },
        { key: 'validator_addr', label: 'Validator' },
      ],
      events.map((e) => ({
        l1_block_number: e.l1_block_number,
        validator_addr: bytesToHex(e.validator_addr),
      })) as Record<string, string | number>[],
      undefined,
      undefined,
      undefined,
      range,
      (r) => openSlashingEventsTable(r),
    );
  };

  const openForcedInclusionsTable = async (range: TimeRange = timeRange) => {
    setTableLoading(true);
    setTimeRange(range);
    const eventsRes = await fetchForcedInclusionEvents(range);
    const events = (eventsRes.data || []) as ForcedInclusionEvent[];
    setTableUrl('forced-inclusions', { range });
    openTable(
      'Forced Inclusions',
      [{ key: 'blob_hash', label: 'Blob Hash' }],
      events.map((e) => ({
        blob_hash: bytesToHex(e.blob_hash),
      })) as Record<string, string | number>[],
      undefined,
      undefined,
      undefined,
      range,
      (r) => openForcedInclusionsTable(r),
    );
  };

  const openActiveGatewaysTable = async (range: TimeRange = timeRange) => {
    setTableLoading(true);
    setTimeRange(range);
    const gatewaysRes = await fetchActiveGatewayAddresses(range);
    setTableUrl('gateways', { range });
    openTable(
      'Active Gateways',
      [{ key: 'address', label: 'Address' }],
      (gatewaysRes.data || []).map((g) => ({ address: g })) as Record<
        string,
        string | number
      >[],
      undefined,
      undefined,
      undefined,
      range,
      (r) => openActiveGatewaysTable(r),
    );
  };

  const openBlobsPerBatchTable = async (range: TimeRange = timeRange) => {
    setTableLoading(true);
    setTimeRange(range);
    const blobsRes = await fetchBatchBlobCounts(range);
    const data = blobsRes.data || [];
    setTableUrl('blobs-per-batch', { range });
    openTable(
      'Blobs per Batch',
      [
        { key: 'batch', label: 'Batch' },
        { key: 'blobs', label: 'Blobs' },
      ],
      data as unknown as Record<string, string | number>[],
      undefined,
      undefined,
      undefined,
      range,
      (r) => openBlobsPerBatchTable(r),
    );
  };

  const openProveTimeTable = async (range: TimeRange = timeRange) => {
    setTableLoading(true);
    setTimeRange(range);
    const res = await fetchProveTimes(range);
    const data = res.data || [];
    setTableUrl('prove-time', { range });
    openTable(
      'Prove Time',
      [
        { key: 'name', label: 'Batch' },
        { key: 'value', label: 'Seconds' },
      ],
      data as unknown as Record<string, string | number>[],
      undefined,
      undefined,
      undefined,
      range,
      (r) => openProveTimeTable(r),
      <BatchProcessChart data={data} lineColor={TAIKO_PINK} />,
    );
  };

  const openVerifyTimeTable = async (range: TimeRange = timeRange) => {
    setTableLoading(true);
    setTimeRange(range);
    const res = await fetchVerifyTimes(range);
    const data = res.data || [];
    setTableUrl('verify-time', { range });
    openTable(
      'Verify Time',
      [
        { key: 'name', label: 'Batch' },
        { key: 'value', label: 'Seconds' },
      ],
      data as unknown as Record<string, string | number>[],
      undefined,
      undefined,
      undefined,
      range,
      (r) => openVerifyTimeTable(r),
      <BatchProcessChart data={data} lineColor="#5DA5DA" />,
    );
  };

  const openBlockTxTable = async (range: TimeRange = timeRange) => {
    setTableLoading(true);
    setTimeRange(range);
    const res = await fetchBlockTransactions(range, 50);
    const data = res.data || [];
    setTableUrl('block-tx', { range });
    openTable(
      'Tx Count Per Block',
      [
        { key: 'block', label: 'Block Number' },
        { key: 'txs', label: 'Tx Count' },
        { key: 'sequencer', label: 'Sequencer' },
      ],
      data as unknown as Record<string, string | number>[],
      undefined,
      undefined,
      undefined,
      range,
      (r) => openBlockTxTable(r),
      <BlockTxChart data={data} barColor="#4E79A7" />,
    );
  };

  const openL2BlockTimesTable = async (range: TimeRange = timeRange) => {
    setTableLoading(true);
    setTimeRange(range);
    const res = await fetchL2BlockTimes(range, selectedSequencer ?? undefined);
    const data = res.data || [];
    setTableUrl('l2-block-times', { range });
    openTable(
      'L2 Block Times',
      [
        { key: 'value', label: 'Block Number' },
        { key: 'timestamp', label: 'Interval (ms)' },
      ],
      data as unknown as Record<string, string | number>[],
      undefined,
      undefined,
      undefined,
      range,
      (r) => openL2BlockTimesTable(r),
      <BlockTimeChart data={data} lineColor="#FAA43A" />,
    );
  };

  const openL1BlockTimesTable = async (range: TimeRange = timeRange) => {
    setTableLoading(true);
    setTimeRange(range);
    const res = await fetchL1BlockTimes(range);
    const data = res.data || [];
    setTableUrl('l1-block-times', { range });
    openTable(
      'L1 Block Times',
      [
        { key: 'value', label: 'Block Number' },
        { key: 'timestamp', label: 'Interval (ms)' },
      ],
      data as unknown as Record<string, string | number>[],
      undefined,
      undefined,
      undefined,
      range,
      (r) => openL1BlockTimesTable(r),
      <BlockTimeChart data={data} lineColor="#60BD68" />,
    );
  };

  const openTpsTable = () => {
    setTableLoading(true);
    setTableUrl('tps');
    const intervalMap = new Map<number, number>();
    l2BlockTimeData.forEach((d) => {
      intervalMap.set(d.value, d.timestamp);
    });
    const data = blockTxData
      .map((b) => {
        const ms = intervalMap.get(b.block);
        if (!ms) return null;
        return { block: b.block, tps: b.txs / (ms / 1000) };
      })
      .filter((d): d is { block: number; tps: number } => d !== null);
    openTable(
      'Transactions Per Second',
      [
        { key: 'block', label: 'Block Number' },
        { key: 'tps', label: 'TPS' },
      ],
      data.map((d) => ({ block: d.block, tps: d.tps.toFixed(2) })) as Record<
        string,
        string | number
      >[],
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      <TpsChart data={data} lineColor="#4E79A7" />,
    );
  };

  const openSequencerDistributionTable = async (
    range: TimeRange,
    page = seqDistTxPage,
    startingAfter?: number,
    endingBefore?: number,
  ) => {
    setTableLoading(true);
    setTimeRange(range);
    setSeqDistTxPage(page);
    setTableUrl('sequencer-dist', {
      range,
      page,
      start: startingAfter,
      end: endingBefore,
    });
    const [distRes, txRes] = await Promise.all([
      fetchSequencerDistribution(range),
      fetchBlockTransactions(
        range,
        50,
        startingAfter,
        endingBefore,
        selectedSequencer ? getSequencerAddress(selectedSequencer) : undefined,
      ),
    ]);
    const txData = txRes.data || [];
    const disablePrev = page === 0;
    const disableNext = txData.length < 50;
    const nextCursor =
      txData.length > 0 ? txData[txData.length - 1].block : undefined;
    const prevCursor = txData.length > 0 ? txData[0].block : undefined;
    openTable(
      'Sequencer Distribution',
      [
        { key: 'name', label: 'Sequencer' },
        { key: 'value', label: 'Blocks' },
      ],
      (distRes.data || []) as unknown as Record<string, string | number>[],
      (row) => openSequencerBlocks(row.name as string, range),
      undefined,
      {
        title: 'Transactions',
        columns: [
          { key: 'block', label: 'Block Number' },
          { key: 'txs', label: 'Tx Count' },
          { key: 'sequencer', label: 'Sequencer' },
        ],
        rows: (txRes.data || []) as unknown as Record<
          string,
          string | number
        >[],
        pagination: {
          page,
          onPrev: () =>
            openSequencerDistributionTable(
              range,
              page - 1,
              undefined,
              prevCursor,
            ),
          onNext: () =>
            openSequencerDistributionTable(
              range,
              page + 1,
              nextCursor,
              undefined,
            ),
          disablePrev,
          disableNext,
        },
      },
      range,
      (r) => openSequencerDistributionTable(r, 0),
    );
  };

  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    if (params.get('view') !== 'table') return;
    setTableLoading(true);
    const table = params.get('table');
    switch (table) {
      case 'sequencer-blocks': {
        const addr = params.get('address');
        const range = (params.get('range') as TimeRange) || timeRange;
        if (addr) void openSequencerBlocks(addr, range);
        break;
      }
      case 'reorgs': {
        const range = (params.get('range') as TimeRange) || timeRange;
        void openL2ReorgsTable(range);
        break;
      }
      case 'slashings': {
        const range = (params.get('range') as TimeRange) || timeRange;
        void openSlashingEventsTable(range);
        break;
      }
      case 'forced-inclusions': {
        const range = (params.get('range') as TimeRange) || timeRange;
        void openForcedInclusionsTable(range);
        break;
      }
      case 'gateways': {
        const range = (params.get('range') as TimeRange) || timeRange;
        void openActiveGatewaysTable(range);
        break;
      }
      case 'prove-time': {
        const range = (params.get('range') as TimeRange) || timeRange;
        void openProveTimeTable(range);
        break;
      }
      case 'verify-time': {
        const range = (params.get('range') as TimeRange) || timeRange;
        void openVerifyTimeTable(range);
        break;
      }
      case 'block-tx': {
        const range = (params.get('range') as TimeRange) || timeRange;
        void openBlockTxTable(range);
        break;
      }
      case 'blobs-per-batch': {
        const range = (params.get('range') as TimeRange) || timeRange;
        void openBlobsPerBatchTable(range);
        break;
      }
      case 'l2-block-times': {
        const range = (params.get('range') as TimeRange) || timeRange;
        void openL2BlockTimesTable(range);
        break;
      }
      case 'l1-block-times': {
        const range = (params.get('range') as TimeRange) || timeRange;
        void openL1BlockTimesTable(range);
        break;
      }
      case 'tps':
        void openTpsTable();
        break;
      case 'sequencer-dist': {
        const range = (params.get('range') as TimeRange) || timeRange;
        const page = parseInt(params.get('page') ?? '0', 10);
        const start = params.get('start');
        const end = params.get('end');
        void openSequencerDistributionTable(
          range,
          page,
          start ? Number(start) : undefined,
          end ? Number(end) : undefined,
        );
        break;
      }
      default:
        break;
    }
  }, []);

  if (tableView) {
    return (
      <DataTable
        title={tableView.title}
        columns={tableView.columns}
        rows={tableView.rows}
        onBack={() => {
          clearTableUrl();
          setTableView(null);
        }}
        onRowClick={tableView.onRowClick}
        extraAction={tableView.extraAction}
        extraTable={tableView.extraTable}
        timeRange={tableView.timeRange}
        onTimeRangeChange={tableView.onTimeRangeChange}
        chart={tableView.chart}
      />
    );
  }

  if (tableLoading) {
    return <div className="p-4">Loading...</div>;
  }

  return (
    <div
      className="min-h-screen bg-white text-gray-800 p-4 md:p-6 lg:p-8"
      style={{ fontFamily: "'Inter', sans-serif" }}
    >
      <DashboardHeader
        timeRange={timeRange}
        onTimeRangeChange={setTimeRange}
        refreshRate={refreshRate}
        onRefreshRateChange={setRefreshRate}
        lastRefresh={lastRefresh}
        onManualRefresh={handleManualRefresh}
        sequencers={sequencerList}
        selectedSequencer={selectedSequencer}
        onSequencerChange={setSelectedSequencer}
      />

      {errorMessage && (
        <div className="mt-4 p-3 bg-red-50 border border-red-200 text-red-700 rounded">
          {errorMessage}
        </div>
      )}

      <main className="mt-6">
        {/* Metrics Grid */}
        {(loadingMetrics ? Object.keys(skeletonGroupCounts) : groupOrder).map(
          (group) =>
            loadingMetrics ? (
              <React.Fragment key={group}>
                {group !== 'Other' && (
                  <h2 className="mt-6 mb-2 text-lg font-semibold">{group}</h2>
                )}
                <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-5 xl:grid-cols-6 2xl:grid-cols-8 gap-4 md:gap-6">
                  {Array.from({ length: skeletonGroupCounts[group] }).map(
                    (_, idx) => (
                      <MetricCardSkeleton key={`${group}-s-${idx}`} />
                    ),
                  )}
                </div>
              </React.Fragment>
            ) : groupedMetrics[group] && groupedMetrics[group].length > 0 ? (
              <React.Fragment key={group}>
                {group !== 'Other' && (
                  <h2 className="mt-6 mb-2 text-lg font-semibold">{group}</h2>
                )}
                <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-5 xl:grid-cols-6 2xl:grid-cols-8 gap-4 md:gap-6">
                  {groupedMetrics[group].map((m, idx) => (
                    <MetricCard
                      key={`${group}-${idx}`}
                      title={m.title}
                      value={m.value}
                      onMore={
                        typeof m.title === 'string' && m.title === 'Avg. L2 TPS'
                          ? () => openTpsTable()
                          : typeof m.title === 'string' && m.title === 'L2 Reorgs'
                            ? () => openL2ReorgsTable()
                            : typeof m.title === 'string' &&
                              m.title === 'Slashing Events'
                              ? () => openSlashingEventsTable()
                              : typeof m.title === 'string' &&
                                m.title === 'Forced Inclusions'
                                ? () => openForcedInclusionsTable()
                                : typeof m.title === 'string' &&
                                  m.title === 'Active Gateways'
                                  ? () => openActiveGatewaysTable()
                                  : undefined
                      }
                    />
                  ))}
                </div>
              </React.Fragment>
            ) : null,
        )}

        {/* Charts Grid - Reordered: Sequencer Pie Chart first */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 md:gap-6 mt-6">
          <ChartCard
            title="Sequencer Distribution"
            onMore={() => openSequencerDistributionTable(timeRange, 0)}
            loading={loadingMetrics}
          >
            <SequencerPieChart
              data={sequencerDistribution.filter(
                (d) => !selectedSequencer || d.name === selectedSequencer,
              )}
            />
          </ChartCard>
          <ChartCard
            title="Prove Time"
            onMore={() => openProveTimeTable(timeRange)}
            loading={loadingMetrics}
          >
            <BatchProcessChart
              data={secondsToProveData}
              lineColor={TAIKO_PINK}
            />
          </ChartCard>
          <ChartCard
            title="Verify Time"
            onMore={() => openVerifyTimeTable(timeRange)}
            loading={loadingMetrics}
          >
            <BatchProcessChart data={secondsToVerifyData} lineColor="#5DA5DA" />
          </ChartCard>
          <ChartCard title="Gas Used Per Block" loading={loadingMetrics}>
            <GasUsedChart data={l2GasUsedData} lineColor="#E573B5" />
          </ChartCard>
          <ChartCard
            title="Tx Count Per Block"
            onMore={() => openBlockTxTable(timeRange)}
            loading={loadingMetrics}
          >
            <BlockTxChart data={blockTxData} barColor="#4E79A7" />
          </ChartCard>
          <ChartCard
            title="Blobs per Batch"
            onMore={() => openBlobsPerBatchTable(timeRange)}
            loading={loadingMetrics}
          >
            <BlobsPerBatchChart data={batchBlobCounts} barColor="#A0CBE8" />
          </ChartCard>
          <ChartCard
            title="L2 Block Times"
            onMore={() => openL2BlockTimesTable(timeRange)}
            loading={loadingMetrics}
          >
            <BlockTimeChart data={l2BlockTimeData} lineColor="#FAA43A" />
          </ChartCard>
          <ChartCard
            title="L1 Block Times"
            onMore={() => openL1BlockTimesTable(timeRange)}
            loading={loadingMetrics}
          >
            <BlockTimeChart data={l1BlockTimeData} lineColor="#60BD68" />
          </ChartCard>
        </div>
      </main>

      {/* Footer for Block Numbers */}
      <footer className="mt-8 pt-6 border-t border-gray-200">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-center md:text-left">
          <div>
            <span className="text-sm text-gray-500">L2 Head Block</span>
            <p className="text-2xl font-semibold" style={{ color: TAIKO_PINK }}>
              {l2HeadBlock}
            </p>
          </div>
          <div>
            <span className="text-sm text-gray-500">L1 Head Block</span>
            <p className="text-2xl font-semibold" style={{ color: TAIKO_PINK }}>
              {l1HeadBlock}
            </p>
          </div>
        </div>
        {/* Copyright notice removed as per request */}
      </footer>
    </div>
  );
};

export default App;
