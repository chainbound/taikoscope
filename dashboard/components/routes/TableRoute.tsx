import React, { useEffect, useState, useCallback, useRef } from 'react';
import { useParams, useSearchParams, useOutletContext } from 'react-router-dom';
import { useRouterNavigation } from '../../hooks/useRouterNavigation';
import { TableView } from '../views/TableView';
import { DashboardHeader } from '../DashboardHeader';
import { TableViewState } from '../../hooks/useTableActions';
import { TimeRange } from '../../types';
import { TABLE_CONFIGS } from '../../config/tableConfig';
import { getSequencerAddress } from '../../sequencerConfig';
import { useDataFetcher } from '../../hooks/useDataFetcher';

interface DashboardContextType {
  timeRange: TimeRange;
  setTimeRange: (range: TimeRange) => void;
  selectedSequencer: string | null;
  setSelectedSequencer: (seq: string | null) => void;
  sequencerList: string[];
  chartsData: any;
  metricsData: any;
  refreshTimer: any;
}

export const TableRoute: React.FC = () => {
  const { tableType } = useParams<{ tableType: string }>();
  const [searchParams, setSearchParams] = useSearchParams();
  const { navigateToDashboard } = useRouterNavigation();

  const {
    timeRange,
    setTimeRange,
    selectedSequencer,
    setSelectedSequencer,
    sequencerList,
    chartsData,
    metricsData,
    refreshTimer,
  } = useOutletContext<DashboardContextType>();

  // Override setTimeRange to update URL params for table route instead of navigating away
  const handleTimeRangeChange = useCallback(
    (newRange: TimeRange) => {
      const newParams = new URLSearchParams(searchParams);
      if (newRange === '1h') {
        newParams.delete('range');
      } else {
        newParams.set('range', newRange);
      }
      setSearchParams(newParams, { replace: true });
      setTimeRange(newRange);
    },
    [searchParams, setSearchParams, setTimeRange],
  );

  const [tableView, setTableView] = useState<TableViewState | undefined>(
    undefined,
  );
  const [tableLoading, setTableLoading] = useState(false);
  const fetchIdRef = useRef(0);

  // Get current time range from URL params, fallback to context
  const currentTimeRange =
    (searchParams.get('range') as TimeRange) || timeRange;

  const { handleManualRefresh } = useDataFetcher({
    timeRange: currentTimeRange,
    selectedSequencer,
    tableView: tableView || null,
    updateChartsData: chartsData.updateChartsData,
    setMetrics: metricsData.setMetrics,
    setLoadingMetrics: metricsData.setLoadingMetrics,
    setErrorMessage: metricsData.setErrorMessage,
    isEconomicsView: metricsData.isEconomicsView,
    refreshRate: refreshTimer.refreshRate,
    updateLastRefresh: refreshTimer.updateLastRefresh,
  });

  useEffect(() => {
    const currentFetchId = ++fetchIdRef.current;

    const loadTable = async () => {
      if (!tableType) return;

      setTableLoading(true);
      setTableView(undefined);

      try {
        const range = currentTimeRange;

        // Handle all tables using config
        const config = TABLE_CONFIGS[tableType];
        if (!config) {
          throw new Error(`Unknown table type: ${tableType}`);
        }

        const fetcherArgs: any[] = [];
        const extraParams: Record<string, any> = {};

        if (tableType === 'sequencer-blocks') {
          const address = searchParams.get('address');
          if (address) {
            fetcherArgs.push(address);
            extraParams.address = address;
          }
        } else if (
          ['l2-block-times', 'l2-gas-used', 'l2-tps'].includes(tableType)
        ) {
          fetcherArgs.push(
            selectedSequencer
              ? getSequencerAddress(selectedSequencer)
              : undefined,
          );
        }

        const res = await config.fetcher(range, ...fetcherArgs);
        if (currentFetchId !== fetchIdRef.current) return;
        let data = res.data || [];
        if (config.reverseOrder) {
          data = [...data].reverse();
        }

        const title =
          typeof config.title === 'function'
            ? config.title(extraParams)
            : config.title;

        const mappedData = config.mapData
          ? config.mapData(data, extraParams)
          : data;
        const chart = config.chart ? config.chart(data) : undefined;

        if (currentFetchId === fetchIdRef.current) {
          setTableView({
            title,
            description: config.description,
            columns: config.columns,
            rows: mappedData,
            chart,
          });
        }
      } catch (error) {
        console.error('Failed to load table:', error);
        metricsData.setErrorMessage(
          `Failed to load ${tableType} table. Please try again.`,
        );
      } finally {
        setTableLoading(false);
      }
    };

    loadTable();

    return () => {
      fetchIdRef.current++;
    };
  }, [
    tableType,
    searchParams,
    currentTimeRange,
    selectedSequencer,
    metricsData.setErrorMessage,
  ]);

  const handleBack = () => {
    navigateToDashboard(true);
  };

  if (!tableView && !tableLoading) {
    return <div>Table not found</div>;
  }

  return (
    <>
      <DashboardHeader
        timeRange={currentTimeRange}
        onTimeRangeChange={handleTimeRangeChange}
        refreshRate={refreshTimer.refreshRate}
        onRefreshRateChange={refreshTimer.setRefreshRate}
        lastRefresh={refreshTimer.lastRefresh}
        onManualRefresh={handleManualRefresh}
        sequencers={sequencerList}
        selectedSequencer={selectedSequencer}
        onSequencerChange={setSelectedSequencer}
      />
      <TableView
        tableView={tableView}
        tableLoading={tableLoading}
        isNavigating={false}
        onBack={handleBack}
      />
    </>
  );
};
