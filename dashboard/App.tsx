import React, { useState, useEffect, useCallback } from 'react';
import { createMetrics, hasBadRequest } from './helpers';
import { DashboardHeader } from './components/DashboardHeader';
import { MetricCard } from './components/MetricCard';
import { ChartCard } from './components/ChartCard';
import { DataTable } from './components/DataTable';
import { SequencerPieChart } from './components/SequencerPieChart';
import { BlockTimeChart } from './components/BlockTimeChart';
import { BatchProcessChart } from './components/BatchProcessChart';
import { GasUsedChart } from './components/GasUsedChart';
import { ReorgDepthChart } from './components/ReorgDepthChart';
import { BlockTxChart } from './components/BlockTxChart';
import { BlobsPerBatchChart } from './components/BlobsPerBatchChart';
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
  type BlockTransaction,
  type BatchBlobCount,
} from './services/apiService';

// Updated Taiko Pink
const TAIKO_PINK = '#e81899';

const App: React.FC = () => {
  const [timeRange, setTimeRange] = useState<TimeRange>('1h');
  const [selectedSequencer, setSelectedSequencer] = useState<string | null>(null);
  const [metrics, setMetrics] = useState<MetricData[]>([]);
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
  const [errorMessage, setErrorMessage] = useState<string>('');
  const [seqDistTxPage, setSeqDistTxPage] = useState<number>(0);
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
        pollId = setInterval(updateHeads, 10000);
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
    ] = await Promise.all([
      fetchL2BlockCadence(range, selectedSequencer ?? undefined),
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
      fetchL2BlockTimes(range, selectedSequencer ?? undefined),
      fetchL2GasUsed(range, selectedSequencer ?? undefined),
      fetchSequencerDistribution(range),
      fetchBlockTransactions(range, 50, undefined, undefined, selectedSequencer ?? undefined),
      fetchBatchBlobCounts(range),
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
    const sequencerDist = (sequencerDistRes.data || []).filter(
      (d) => !selectedSequencer || d.name === selectedSequencer,
    );
    const txPerBlock = blockTxRes.data || [];
    const blobsPerBatch = batchBlobCountsRes.data || [];

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
  }, [timeRange, selectedSequencer]);

  useEffect(() => {
    saveRefreshRate(refreshRate);
  }, [refreshRate]);

  useEffect(() => {
    if (tableView) return;
    fetchData();
    const interval = setInterval(fetchData, Math.max(refreshRate, 10000));
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
  };

  const openSequencerBlocks = async (
    address: string,
    range: TimeRange = timeRange,
  ) => {
    setTimeRange(range);
    const blocksRes = await fetchSequencerBlocks(range, address);
    setTableUrl('sequencer-blocks', { address });
    openTable(
      `Blocks proposed by ${address}`,
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
    setTimeRange(range);
    const eventsRes = await fetchL2ReorgEvents(range);
    const events = (eventsRes.data || []) as L2ReorgEvent[];
    setTableUrl('reorgs');
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
    setTimeRange(range);
    const eventsRes = await fetchSlashingEvents(range);
    const events = (eventsRes.data || []) as SlashingEvent[];
    setTableUrl('slashings');
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
    setTimeRange(range);
    const eventsRes = await fetchForcedInclusionEvents(range);
    const events = (eventsRes.data || []) as ForcedInclusionEvent[];
    setTableUrl('forced-inclusions');
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
    setTimeRange(range);
    const gatewaysRes = await fetchActiveGatewayAddresses(range);
    setTableUrl('gateways');
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
    setTimeRange(range);
    const countsRes = await fetchBatchBlobCounts(range);
    openTable(
      'Blobs per Batch',
      [
        { key: 'batch', label: 'Batch' },
        { key: 'blobs', label: 'Blobs' },
      ],
      (countsRes.data || []) as unknown as Record<string, string | number>[],
      undefined,
      undefined,
      undefined,
      range,
      (r) => openBlobsPerBatchTable(r),
    );
  };

  const openProveTimeTable = async (range: TimeRange = timeRange) => {
    setTimeRange(range);
    const res = await fetchProveTimes(range);
    openTable(
      'Prove Time',
      [
        { key: 'name', label: 'Batch' },
        { key: 'value', label: 'Seconds' },
      ],
      (res.data || []) as unknown as Record<string, string | number>[],
      undefined,
      undefined,
      undefined,
      range,
      (r) => openProveTimeTable(r),
    );
  };

  const openVerifyTimeTable = async (range: TimeRange = timeRange) => {
    setTimeRange(range);
    const res = await fetchVerifyTimes(range);
    openTable(
      'Verify Time',
      [
        { key: 'name', label: 'Batch' },
        { key: 'value', label: 'Seconds' },
      ],
      (res.data || []) as unknown as Record<string, string | number>[],
      undefined,
      undefined,
      undefined,
      range,
      (r) => openVerifyTimeTable(r),
    );
  };

  const openTxCountPerBlockTable = async (range: TimeRange = timeRange) => {
    setTimeRange(range);
    const res = await fetchBlockTransactions(range, 50);
    openTable(
      'Tx Count Per Block',
      [
        { key: 'block', label: 'Block Number' },
        { key: 'txs', label: 'Tx Count' },
        { key: 'sequencer', label: 'Sequencer' },
      ],
      (res.data || []) as unknown as Record<string, string | number>[],
      undefined,
      undefined,
      undefined,
      range,
      (r) => openTxCountPerBlockTable(r),
    );
  };

  const openL2BlockTimesTable = async (range: TimeRange = timeRange) => {
    setTimeRange(range);
    const res = await fetchL2BlockTimes(range, selectedSequencer ?? undefined);
    openTable(
      'L2 Block Times',
      [
        { key: 'value', label: 'Block Number' },
        { key: 'timestamp', label: 'Interval (ms)' },
      ],
      (res.data || []) as unknown as Record<string, string | number>[],
      undefined,
      undefined,
      undefined,
      range,
      (r) => openL2BlockTimesTable(r),
    );
  };

  const openL1BlockTimesTable = async (range: TimeRange = timeRange) => {
    setTimeRange(range);
    const res = await fetchL1BlockTimes(range);
    openTable(
      'L1 Block Times',
      [
        { key: 'value', label: 'Block Number' },
        { key: 'timestamp', label: 'Interval (ms)' },
      ],
      (res.data || []) as unknown as Record<string, string | number>[],
      undefined,
      undefined,
      undefined,
      range,
      (r) => openL1BlockTimesTable(r),
    );
  };

  const openSequencerDistributionTable = async (
    range: TimeRange,
    page = seqDistTxPage,
    startingAfter?: number,
    endingBefore?: number,
  ) => {
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
        selectedSequencer ?? undefined,
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
        { key: 'name', label: 'Address' },
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
    const table = params.get('table');
    switch (table) {
      case 'sequencer-blocks': {
        const addr = params.get('address');
        if (addr) void openSequencerBlocks(addr, timeRange);
        break;
      }
      case 'reorgs':
        void openL2ReorgsTable(timeRange);
        break;
      case 'slashings':
        void openSlashingEventsTable(timeRange);
        break;
      case 'forced-inclusions':
        void openForcedInclusionsTable(timeRange);
        break;
      case 'gateways':
        void openActiveGatewaysTable(timeRange);
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
        {groupOrder.map((group) =>
          groupedMetrics[group] && groupedMetrics[group].length > 0 ? (
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
                      typeof m.title === 'string' && m.title === 'L2 Reorgs'
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
          >
            <SequencerPieChart data={sequencerDistribution} />
          </ChartCard>
          <ChartCard
            title="Prove Time"
            onMore={() => openProveTimeTable()}
          >
            <BatchProcessChart
              data={secondsToProveData}
              lineColor={TAIKO_PINK}
            />
          </ChartCard>
          <ChartCard
            title="Verify Time"
            onMore={() => openVerifyTimeTable()}
          >
            <BatchProcessChart data={secondsToVerifyData} lineColor="#5DA5DA" />
          </ChartCard>
          <ChartCard title="Gas Used Per Block">
            <GasUsedChart data={l2GasUsedData} lineColor="#E573B5" />
          </ChartCard>
          <ChartCard
            title="Tx Count Per Block"
            onMore={() => openTxCountPerBlockTable()}
          >
            <BlockTxChart data={blockTxData} barColor="#4E79A7" />
          </ChartCard>
          <ChartCard title="Blobs per Batch" onMore={() => openBlobsPerBatchTable()}>
            <BlobsPerBatchChart data={batchBlobCounts} barColor="#A0CBE8" />
          </ChartCard>
          <ChartCard
            title="L2 Block Times"
            onMore={() => openL2BlockTimesTable()}
          >
            <BlockTimeChart data={l2BlockTimeData} lineColor="#FAA43A" />
          </ChartCard>
          <ChartCard
            title="L1 Block Times"
            onMore={() => openL1BlockTimesTable()}
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
