import React, { useState, useEffect, useCallback } from 'react';
import { useMetricsData } from './hooks/useMetricsData';
import { useChartsData } from './hooks/useChartsData';
import { useBlockData } from './hooks/useBlockData';
import { useRefreshTimer } from './hooks/useRefreshTimer';
import { DashboardView } from './components/views/DashboardView';
import { TableView } from './components/views/TableView';
import { useTableActions } from './hooks/useTableActions';
import { useSearchParams } from './hooks/useSearchParams';
import {
  TimeRange,
} from './types';

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
  }, [timeRange, fetchData, refreshTimer.refreshRate, searchParams]);

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
      <TableView
        tableView={tableView}
        tableLoading={tableLoading}
        isNavigating={searchParams.navigationState.isNavigating}
        refreshTimer={refreshTimer}
        sequencerList={sequencerList}
        selectedSequencer={selectedSequencer}
        onSequencerChange={handleSequencerChange}
        onBack={handleBack}
        onManualRefresh={handleManualRefresh}
      />
    );
  }

  return (
    <DashboardView
      timeRange={timeRange}
      onTimeRangeChange={setTimeRange}
      selectedSequencer={selectedSequencer}
      onSequencerChange={handleSequencerChange}
      sequencerList={sequencerList}
      metricsData={metricsData}
      chartsData={chartsData}
      blockData={blockData}
      refreshTimer={refreshTimer}
      onManualRefresh={handleManualRefresh}
      onOpenTable={openGenericTable}
      onOpenTpsTable={openTpsTable}
      onOpenSequencerDistributionTable={openSequencerDistributionTable}
    />
  );
};

export default App;
