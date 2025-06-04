import React, { useState, useEffect, useCallback, lazy } from 'react';
import { useMetricsData } from './hooks/useMetricsData';
import { useChartsData } from './hooks/useChartsData';
import { useBlockData } from './hooks/useBlockData';
import { useRefreshTimer } from './hooks/useRefreshTimer';
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
  MetricData,
} from './types';
import { TAIKO_PINK } from './theme';

const App: React.FC = () => {
  const searchParams = useSearchParams();
  const [timeRange, setTimeRange] = useState<TimeRange>('1h');
  const [selectedSequencer, setSelectedSequencer] = useState<string | null>(
    searchParams.get('sequencer'),
  );

  // Use new hooks for data management
  const metricsData = useMetricsData();
  const chartsData = useChartsData();
  const blockData = useBlockData();
  const refreshTimer = useRefreshTimer();

  const sequencerList = React.useMemo(
    () => chartsData.sequencerDistribution.map((s) => s.name),
    [chartsData.sequencerDistribution],
  );
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
    chartsData.blockTxData,
    chartsData.l2BlockTimeData,
  );

  useEffect(() => {
    const seq = searchParams.get('sequencer');
    setSelectedSequencer(seq ?? null);
  }, [searchParams]);

  const handleSequencerChange = useCallback((seq: string | null) => {
    try {
      setSelectedSequencer(seq);
      const url = new URL(window.location.href);
      if (seq) {
        url.searchParams.set('sequencer', seq);
      } else {
        url.searchParams.delete('sequencer');
      }
      searchParams.navigate(url);
    } catch (err) {
      console.error('Failed to handle sequencer change:', err);
      // Fallback: just update state without navigation
      setSelectedSequencer(seq);
    }
  }, [searchParams]);

  // Update metrics with current block heads whenever they change
  useEffect(() => {
    if (metricsData.metrics.length > 0) {
      const updatedMetrics = blockData.updateMetricsWithBlockHeads(metricsData.metrics);
      metricsData.setMetrics(updatedMetrics);
    }
  }, [blockData.l1HeadBlock, blockData.l2HeadBlock]);


  const fetchData = useCallback(async () => {
    refreshTimer.updateLastRefresh();

    const result = await metricsData.fetchMetricsData(timeRange, selectedSequencer);

    // Update charts data if available (main dashboard view)
    if (result?.chartData) {
      chartsData.updateChartsData(result.chartData);
    }
  }, [timeRange, selectedSequencer, metricsData, chartsData, refreshTimer]);

  const handleManualRefresh = useCallback(() => {
    if (tableView && tableView.onRefresh) {
      // If we're in a table view and it has a refresh function, use that
      tableView.onRefresh();
    } else {
      // Otherwise refresh the main dashboard data
      void fetchData();
    }
  }, [fetchData, tableView?.onRefresh]); // Only depend on the onRefresh function, not the entire tableView

  useEffect(() => {
    const isTableView = tableView || searchParams.get('view') === 'table';
    if (isTableView) return;
    fetchData();
    const interval = setInterval(fetchData, Math.max(refreshTimer.refreshRate, 60000));
    return () => clearInterval(interval);
  }, [timeRange, fetchData, refreshTimer.refreshRate, searchParams]); // Remove tableView dependency

  const isEconomicsView = searchParams.get('view') === 'economics';

  const visibleMetrics = React.useMemo(
    () =>
      metricsData.metrics.filter((m) => {
        if (selectedSequencer && m.group === 'Sequencers') return false;
        if (isEconomicsView) return m.group === 'Network Economics';
        return m.group !== 'Network Economics';
      }),
    [metricsData.metrics, selectedSequencer, isEconomicsView],
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
      'Network Health': 4,
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
    try {
      const params = searchParams;
      if (params.get('view') !== 'table') {
        setTableView(null);
        return;
      }

      // If we already have a table view and it matches the current URL state, don't reload
      const table = params.get('table');
      const range = (params.get('range') as TimeRange) || timeRange;

      if (tableView && tableView.timeRange === range) {
        return;
      }

      setTableLoading(true);

      // Add error boundary for table operations
      const handleTableError = (tableName: string, error: any) => {
        console.error(`Failed to open ${tableName} table:`, error);
        setTableLoading(false);
        metricsData.setErrorMessage(`Failed to load ${tableName} table. Please try again.`);
      };

      switch (table) {
        case 'sequencer-blocks': {
          const addr = params.get('address');
          if (addr) {
            openGenericTable('sequencer-blocks', range, { address: addr })
              .catch((err) => handleTableError('sequencer-blocks', err));
          } else {
            setTableLoading(false);
          }
          break;
        }
        case 'tps':
          try {
            openTpsTable();
          } catch (err) {
            handleTableError('TPS', err);
          }
          break;
        case 'sequencer-dist': {
          const pageStr = params.get('page') ?? '0';
          const page = parseInt(pageStr, 10);
          if (isNaN(page) || page < 0) {
            console.warn('Invalid page parameter:', pageStr);
            setTableLoading(false);
            break;
          }
          const start = params.get('start');
          const end = params.get('end');
          openSequencerDistributionTable(
            range,
            page,
            start ? Number(start) : undefined,
            end ? Number(end) : undefined,
          ).catch((err) => handleTableError('sequencer-distribution', err));
          break;
        }
        default: {
          if (table) {
            openGenericTable(table, range)
              .catch((err) => handleTableError(table, err));
          } else {
            setTableLoading(false);
          }
          break;
        }
      }
    } catch (err) {
      console.error('Failed to handle route change:', err);
      setTableLoading(false);
      metricsData.setErrorMessage('Navigation error occurred. Please try again.');
    }
  }, [
    searchParams,
    openGenericTable,
    openTpsTable,
    openSequencerDistributionTable,
    setTableView,
    setTableLoading,
    timeRange,
    tableView,
  ]);

  const handleBack = useCallback(() => {
    try {
      if (searchParams.navigationState.canGoBack) {
        searchParams.goBack();
      } else {
        // Fallback: navigate to dashboard home
        const url = new URL(window.location.href);
        url.searchParams.delete('view');
        url.searchParams.delete('table');
        url.searchParams.delete('address');
        url.searchParams.delete('page');
        url.searchParams.delete('start');
        url.searchParams.delete('end');
        searchParams.navigate(url, true);
      }
      setTableView(null);
    } catch (err) {
      console.error('Failed to handle back navigation:', err);
      // Emergency fallback: just clear the table view
      setTableView(null);
      metricsData.setErrorMessage('Navigation error occurred.');
    }
  }, [searchParams, setTableView]);

  useEffect(() => {
    try {
      handleRouteChange();
    } catch (err) {
      console.error('Route change effect error:', err);
      metricsData.setErrorMessage('Navigation error occurred.');
    }
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
        refreshRate={refreshTimer.refreshRate}
        onRefreshRateChange={refreshTimer.setRefreshRate}
        lastRefresh={refreshTimer.lastRefresh}
        onManualRefresh={handleManualRefresh}
        sequencers={sequencerList}
        selectedSequencer={selectedSequencer}
        onSequencerChange={handleSequencerChange}
        chart={tableView.chart}
        isNavigating={searchParams.navigationState.isNavigating}
      />
    );
  }

  if (tableLoading || searchParams.navigationState.isNavigating) {
    return (
      <div className="p-4">
        <div className="flex items-center space-x-2">
          <div className="animate-spin rounded-full h-4 w-4 border-b-2" style={{ borderColor: TAIKO_PINK }}></div>
          <span>{searchParams.navigationState.isNavigating ? 'Navigating...' : 'Loading...'}</span>
        </div>
      </div>
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
        refreshRate={refreshTimer.refreshRate}
        onRefreshRateChange={refreshTimer.setRefreshRate}
        lastRefresh={refreshTimer.lastRefresh}
        onManualRefresh={handleManualRefresh}
        sequencers={sequencerList}
        selectedSequencer={selectedSequencer}
        onSequencerChange={handleSequencerChange}
      />

      {(metricsData.errorMessage || searchParams.navigationState.lastError) && (
        <div className="mt-4 p-3 bg-red-50 border border-red-200 text-red-700 rounded">
          <div className="flex justify-between items-start">
            <div className="flex-1">
              {metricsData.errorMessage || searchParams.navigationState.lastError}
              {searchParams.navigationState.errorCount > 0 && (
                <div className="text-sm mt-1 text-red-600">
                  Navigation issues detected. Try refreshing the page if problems persist.
                </div>
              )}
            </div>
            <div className="flex space-x-2 ml-4">
              {searchParams.navigationState.errorCount > 2 && (
                <button
                  onClick={() => {
                    try {
                      searchParams.resetNavigation();
                      metricsData.setErrorMessage('');
                    } catch (err) {
                      console.error('Failed to reset navigation:', err);
                    }
                  }}
                  className="text-sm bg-red-600 text-white px-2 py-1 rounded hover:bg-red-700"
                >
                  Reset
                </button>
              )}
              <button
                onClick={() => metricsData.setErrorMessage('')}
                className="text-sm text-red-600 hover:text-red-800"
              >
                ✕
              </button>
            </div>
          </div>
        </div>
      )}

      <main className="mt-6">
        {/* Metrics Grid */}
        {(metricsData.loadingMetrics
          ? Object.keys(displayedSkeletonCounts)
          : displayedGroupOrder
        ).map((group) =>
          metricsData.loadingMetrics ? (
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
                                m.title === 'Missed Proposals'
                                ? () => openGenericTable('missed-proposals')
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
                loading={metricsData.loadingMetrics}
              >
                <SequencerPieChart
                  key={timeRange}
                  data={chartsData.sequencerDistribution}
                />
              </ChartCard>
            )}
            <ChartCard
              title="Prove Time"
              onMore={() => openGenericTable('prove-time', timeRange)}
              loading={metricsData.loadingMetrics}
            >
              <BatchProcessChart
                key={timeRange}
                data={chartsData.secondsToProveData}
                lineColor={TAIKO_PINK}
              />
            </ChartCard>
            <ChartCard
              title="Verify Time"
              onMore={() => openGenericTable('verify-time', timeRange)}
              loading={metricsData.loadingMetrics}
            >
              <BatchProcessChart
                key={timeRange}
                data={chartsData.secondsToVerifyData}
                lineColor="#5DA5DA"
              />
            </ChartCard>
            <ChartCard title="Gas Used Per Block" loading={metricsData.loadingMetrics}>
              <GasUsedChart
                key={timeRange}
                data={chartsData.l2GasUsedData}
                lineColor="#E573B5"
              />
            </ChartCard>
            <ChartCard
              title="Tx Count Per Block"
              onMore={() => openGenericTable('block-tx', timeRange)}
              loading={metricsData.loadingMetrics}
            >
              <BlockTxChart
                key={timeRange}
                data={chartsData.blockTxData}
                barColor="#4E79A7"
              />
            </ChartCard>
            <ChartCard
              title="Blobs per Batch"
              onMore={() => openGenericTable('blobs-per-batch', timeRange)}
              loading={metricsData.loadingMetrics}
            >
              <BlobsPerBatchChart
                key={timeRange}
                data={chartsData.batchBlobCounts}
                barColor="#A0CBE8"
              />
            </ChartCard>
            <ChartCard
              title="L2 Block Times"
              onMore={() => openGenericTable('l2-block-times', timeRange)}
              loading={metricsData.loadingMetrics}
            >
              <BlockTimeChart
                key={timeRange}
                data={chartsData.l2BlockTimeData}
                lineColor="#FAA43A"
              />
            </ChartCard>
            <ChartCard
              title="L1 Block Times"
              onMore={() => openGenericTable('l1-block-times', timeRange)}
              loading={metricsData.loadingMetrics}
            >
              <BlockTimeChart
                key={timeRange}
                data={chartsData.l1BlockTimeData}
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
              {blockData.l2HeadBlock}
            </p>
          </div>
          <div>
            <span className="text-sm text-gray-500">L1 Head Block</span>
            <p className="text-2xl font-semibold" style={{ color: TAIKO_PINK }}>
              {blockData.l1HeadBlock}
            </p>
          </div>
        </div>
        {/* Copyright notice removed as per request */}
      </footer>
    </div>
  );
};

export default App;
