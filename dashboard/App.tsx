import React, { useState, useEffect, useCallback, lazy } from 'react';
import { createMetrics, hasBadRequest } from './helpers';
import { DashboardHeader } from './components/DashboardHeader';
import { MetricCard } from './components/MetricCard';
import { MetricCardSkeleton } from './components/MetricCardSkeleton';
import { ChartCard } from './components/ChartCard';
import { DataTable } from './components/DataTable';
import { useTableActions } from './hooks/useTableActions';
import { useSearchParams } from './hooks/useSearchParams';
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
import {
  TimeRange,
  TimeSeriesData,
  PieChartDataItem,
  MetricData,
} from './types';
import { loadRefreshRate, saveRefreshRate } from './utils';
import { getSequencerAddress } from './sequencerConfig';
import { TAIKO_PINK } from './theme';
import {
  fetchAvgProveTime,
  fetchAvgVerifyTime,
  fetchL2BlockCadence,
  fetchBatchPostingCadence,
  fetchL2Reorgs,
  fetchSlashingEventCount,
  fetchForcedInclusionCount,
  fetchPreconfData,
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
  fetchBlockTransactions,
  fetchBatchBlobCounts,
  fetchL2TxFee,
  fetchAvgL2Tps,
  type BlockTransaction,
  type BatchBlobCount,
} from './services/apiService';

const App: React.FC = () => {
  const searchParams = useSearchParams();
  const [timeRange, setTimeRange] = useState<TimeRange>('1h');
  const [selectedSequencer, setSelectedSequencer] = useState<string | null>(
    searchParams.get('sequencer'),
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
  const {
    tableView,
    tableLoading,
    setTableView,
    setTableLoading,
    openGenericTable,
    openTpsTable,
    openSequencerDistributionTable,
  } = useTableActions(
    timeRange,
    setTimeRange,
    selectedSequencer,
    blockTxData,
    l2BlockTimeData,
  );

  useEffect(() => {
    const seq = searchParams.get('sequencer');
    setSelectedSequencer(seq ?? null);
  }, [searchParams]);

  const handleSequencerChange = useCallback((seq: string | null) => {
    setSelectedSequencer(seq);
    const url = new URL(window.location.href);
    if (seq) {
      url.searchParams.set('sequencer', seq);
    } else {
      url.searchParams.delete('sequencer');
    }
    window.history.pushState(null, '', url);
  }, []);

  useEffect(() => {
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

    updateHeads();
    const pollId = setInterval(updateHeads, 60000);

    return () => {
      clearInterval(pollId);
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
      avgTpsRes,
      preconfRes,
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
      l2TxFeeRes,
    ] = await Promise.all([
      fetchL2BlockCadence(
        range,
        selectedSequencer ? getSequencerAddress(selectedSequencer) : undefined,
      ),
      fetchBatchPostingCadence(range),
      fetchAvgProveTime(range),
      fetchAvgVerifyTime(range),
      fetchAvgL2Tps(
        range,
        selectedSequencer ? getSequencerAddress(selectedSequencer) : undefined,
      ),
      fetchPreconfData(),
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
      fetchL2TxFee(
        range,
        selectedSequencer ? getSequencerAddress(selectedSequencer) : undefined,
      ),
    ]);

    const l2Cadence = l2CadenceRes.data;
    const batchCadence = batchCadenceRes.data;
    const avgProve = avgProveRes.data;
    const avgVerify = avgVerifyRes.data;
    const avgTps = avgTpsRes.data;
    const preconfData = preconfRes.data;
    const activeGateways = preconfData ? preconfData.candidates.length : null;
    const currentOperator = preconfData?.current_operator ?? null;
    const nextOperator = preconfData?.next_operator ?? null;
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
    const l2TxFee = l2TxFeeRes.data;

    const anyBadRequest = hasBadRequest([
      l2CadenceRes,
      batchCadenceRes,
      avgProveRes,
      avgVerifyRes,
      avgTpsRes,
      preconfRes,
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
      l2TxFee,
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
    if (tableView || searchParams.get('view') === 'table') return;
    fetchData();
    const interval = setInterval(fetchData, Math.max(refreshRate, 60000));
    return () => clearInterval(interval);
  }, [timeRange, fetchData, refreshRate, tableView, searchParams]);

  const isEconomicsView = searchParams.get('view') === 'economics';

  const visibleMetrics = React.useMemo(
    () =>
      metrics.filter((m) => {
        if (selectedSequencer && m.group === 'Sequencers') return false;
        if (isEconomicsView) return m.group === 'Network Economics';
        return m.group !== 'Network Economics';
      }),
    [metrics, selectedSequencer, isEconomicsView],
  );

  const groupedMetrics = visibleMetrics.reduce<Record<string, MetricData[]>>(
    (acc, m) => {
      const group = m.group ?? 'Other';
      if (!acc[group]) acc[group] = [];
      acc[group].push(m);
      return acc;
    },
    {},
  );
  const groupOrder = isEconomicsView
    ? ['Network Economics']
    : ['Network Performance', 'Network Health', 'Sequencers', 'Other'];
  const skeletonGroupCounts: Record<string, number> = isEconomicsView
    ? { 'Network Economics': 1 }
    : {
        'Network Performance': 5,
        'Network Health': 3,
        Sequencers: 3,
      };

  const displayGroupName = useCallback(
    (group: string): string => {
      if (!selectedSequencer) return group;
      if (group === 'Network Performance') return 'Sequencer Performance';
      if (group === 'Network Health') return 'Sequencer Health';
      return group;
    },
    [selectedSequencer],
  );
  const displayedGroupOrder = selectedSequencer
    ? groupOrder.filter((g) => g !== 'Sequencers')
    : groupOrder;
  const displayedSkeletonCounts = React.useMemo(
    () =>
      selectedSequencer
        ? { ...skeletonGroupCounts, Sequencers: 0 }
        : skeletonGroupCounts,
    [selectedSequencer, skeletonGroupCounts],
  );

  const handleRouteChange = useCallback(() => {
    const params = searchParams;
    if (params.get('view') !== 'table') {
      setTableView(null);
      return;
    }
    setTableLoading(true);
    const table = params.get('table');
    switch (table) {
      case 'sequencer-blocks': {
        const addr = params.get('address');
        const range = (params.get('range') as TimeRange) || timeRange;
        if (addr)
          void openGenericTable('sequencer-blocks', range, { address: addr });
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
      default: {
        const range = (params.get('range') as TimeRange) || timeRange;
        if (table) void openGenericTable(table, range);
        break;
      }
    }
  }, [
    openGenericTable,
    openTpsTable,
    openSequencerDistributionTable,
    setTableView,
    setTableLoading,
    timeRange,
  ]);

  const handleBack = useCallback(() => {
    const url = new URL(window.location.href);
    url.searchParams.delete('view');
    url.searchParams.delete('table');
    url.searchParams.delete('address');
    url.searchParams.delete('page');
    url.searchParams.delete('start');
    url.searchParams.delete('end');
    window.history.pushState(null, '', url);
    setTableView(null);
  }, [setTableView]);

  useEffect(() => {
    handleRouteChange();
  }, [handleRouteChange, searchParams]);

  if (tableView) {
    return (
      <DataTable
        title={tableView.title}
        description={tableView.description}
        columns={tableView.columns}
        rows={tableView.rows}
        onBack={handleBack}
        onRowClick={tableView.onRowClick}
        extraAction={tableView.extraAction}
        extraTable={tableView.extraTable}
        timeRange={tableView.timeRange}
        onTimeRangeChange={tableView.onTimeRangeChange}
        refreshRate={refreshRate}
        onRefreshRateChange={setRefreshRate}
        lastRefresh={lastRefresh}
        onManualRefresh={handleManualRefresh}
        sequencers={sequencerList}
        selectedSequencer={selectedSequencer}
        onSequencerChange={handleSequencerChange}
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
        onSequencerChange={handleSequencerChange}
      />

      {errorMessage && (
        <div className="mt-4 p-3 bg-red-50 border border-red-200 text-red-700 rounded">
          {errorMessage}
        </div>
      )}

      <main className="mt-6">
        {/* Metrics Grid */}
        {(loadingMetrics
          ? Object.keys(displayedSkeletonCounts)
          : displayedGroupOrder
        ).map((group) =>
          loadingMetrics ? (
            <React.Fragment key={group}>
              {group !== 'Other' && (
                <h2 className="mt-6 mb-2 text-lg font-semibold">
                  {displayGroupName(group)}
                </h2>
              )}
              <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-5 xl:grid-cols-6 2xl:grid-cols-8 gap-4 md:gap-6">
                {Array.from({ length: displayedSkeletonCounts[group] }).map(
                  (_, idx) => (
                    <MetricCardSkeleton key={`${group}-s-${idx}`} />
                  ),
                )}
              </div>
            </React.Fragment>
          ) : groupedMetrics[group] && groupedMetrics[group].length > 0 ? (
            <React.Fragment key={group}>
              {group !== 'Other' && (
                <h2 className="mt-6 mb-2 text-lg font-semibold">
                  {displayGroupName(group)}
                </h2>
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
                          ? () => openGenericTable('reorgs')
                          : typeof m.title === 'string' &&
                              m.title === 'Slashing Events'
                            ? () => openGenericTable('slashings')
                            : typeof m.title === 'string' &&
                                m.title === 'Forced Inclusions'
                              ? () => openGenericTable('forced-inclusions')
                              : typeof m.title === 'string' &&
                                  m.title === 'Active Sequencers'
                                ? () => openGenericTable('gateways')
                                : typeof m.title === 'string' &&
                                    m.title === 'Batch Posting Cadence'
                                  ? () =>
                                      openGenericTable('batch-posting-cadence')
                                  : undefined
                    }
                  />
                ))}
              </div>
            </React.Fragment>
          ) : null,
        )}

        {!isEconomicsView && (
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 md:gap-6 mt-6">
            {!selectedSequencer && (
              <ChartCard
                title="Sequencer Distribution"
                onMore={() => openSequencerDistributionTable(timeRange, 0)}
                loading={loadingMetrics}
              >
                <SequencerPieChart
                  key={timeRange}
                  data={sequencerDistribution}
                />
              </ChartCard>
            )}
            <ChartCard
              title="Prove Time"
              onMore={() => openGenericTable('prove-time', timeRange)}
              loading={loadingMetrics}
            >
              <BatchProcessChart
                key={timeRange}
                data={secondsToProveData}
                lineColor={TAIKO_PINK}
              />
            </ChartCard>
            <ChartCard
              title="Verify Time"
              onMore={() => openGenericTable('verify-time', timeRange)}
              loading={loadingMetrics}
            >
              <BatchProcessChart
                key={timeRange}
                data={secondsToVerifyData}
                lineColor="#5DA5DA"
              />
            </ChartCard>
            <ChartCard title="Gas Used Per Block" loading={loadingMetrics}>
              <GasUsedChart
                key={timeRange}
                data={l2GasUsedData}
                lineColor="#E573B5"
              />
            </ChartCard>
            <ChartCard
              title="Tx Count Per Block"
              onMore={() => openGenericTable('block-tx', timeRange)}
              loading={loadingMetrics}
            >
              <BlockTxChart
                key={timeRange}
                data={blockTxData}
                barColor="#4E79A7"
              />
            </ChartCard>
            <ChartCard
              title="Blobs per Batch"
              onMore={() => openGenericTable('blobs-per-batch', timeRange)}
              loading={loadingMetrics}
            >
              <BlobsPerBatchChart
                key={timeRange}
                data={batchBlobCounts}
                barColor="#A0CBE8"
              />
            </ChartCard>
            <ChartCard
              title="L2 Block Times"
              onMore={() => openGenericTable('l2-block-times', timeRange)}
              loading={loadingMetrics}
            >
              <BlockTimeChart
                key={timeRange}
                data={l2BlockTimeData}
                lineColor="#FAA43A"
              />
            </ChartCard>
            <ChartCard
              title="L1 Block Times"
              onMore={() => openGenericTable('l1-block-times', timeRange)}
              loading={loadingMetrics}
            >
              <BlockTimeChart
                key={timeRange}
                data={l1BlockTimeData}
                lineColor="#60BD68"
              />
            </ChartCard>
          </div>
        )}
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
