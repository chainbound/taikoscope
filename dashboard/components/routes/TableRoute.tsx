import React, { useEffect, useState, useCallback, useRef } from 'react';
import { useParams, useSearchParams, useOutletContext } from 'react-router-dom';
import { useRouterNavigation } from '../../hooks/useRouterNavigation';
import { TableView } from '../views/TableView';
import { DashboardHeader } from '../DashboardHeader';
import { TableViewState } from '../../hooks/useTableActions';
import {
  TimeRange,
  ChartsData,
  MetricsDataState,
  RefreshTimerState,
} from '../../types';
import { TABLE_CONFIGS } from '../../config/tableConfig';
import { getSequencerAddress } from '../../sequencerConfig';
import { useDataFetcher } from '../../hooks/useDataFetcher';

interface DashboardContextType {
  timeRange: TimeRange;
  setTimeRange: (range: TimeRange) => void;
  selectedSequencer: string | null;
  setSelectedSequencer: (seq: string | null) => void;
  sequencerList: string[];
  chartsData: ChartsData;
  metricsData: MetricsDataState;
  refreshTimer: RefreshTimerState;
}

export const TableRoute: React.FC = () => {
  const { tableType } = useParams<{ tableType: string }>();
  const [searchParams, setSearchParams] = useSearchParams();
  const { navigateToDashboard } = useRouterNavigation();

  const PAGE_LIMIT = 50;

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

        const fetcherArgs: (string | number | undefined)[] = [];
        const extraParams: Record<string, string | number | undefined> = {};

        const pageStr = searchParams.get('page') ?? '0';
        const page = parseInt(pageStr, 10);
        const start = searchParams.get('start');
        const end = searchParams.get('end');
        const startingAfter = start ? Number(start) : undefined;
        const endingBefore = end ? Number(end) : undefined;

        if (tableType === 'sequencer-blocks') {
          const address = searchParams.get('address');
          if (address) {
            fetcherArgs.push(address);
            extraParams.address = address;
          }
        } else if (['l2-gas-used', 'l2-tps'].includes(tableType)) {
          fetcherArgs.push(
            selectedSequencer
              ? getSequencerAddress(selectedSequencer)
              : undefined,
          );
        }

        let res;
        let aggRes;
        if (tableType === 'reorgs' || tableType === 'prove-times' || tableType === 'verify-times') {
          if (config.aggregatedFetcher) {
            [res, aggRes] = await Promise.all([
              config.fetcher(range, PAGE_LIMIT, startingAfter, endingBefore),
              config.aggregatedFetcher(range),
            ]);
          } else {
            [res] = await Promise.all([
              config.fetcher(range, PAGE_LIMIT, startingAfter, endingBefore),
            ]);
          }
        } else if (config.supportsPagination) {
          const address = fetcherArgs.pop();
          // For l2-block-times, fetch one extra on first page so slicing off the first still yields PAGE_LIMIT items
          const fetchLimit = tableType === 'l2-block-times' && startingAfter === undefined && endingBefore === undefined
            ? PAGE_LIMIT + 1
            : PAGE_LIMIT;
          if (config.aggregatedFetcher) {
            [res, aggRes] = await Promise.all([
              config.fetcher(
                range,
                address,
                fetchLimit,
                startingAfter,
                endingBefore,
              ),
              config.aggregatedFetcher(range, address),
            ]);
          } else {
            [res] = await Promise.all([
              config.fetcher(
                range,
                address,
                fetchLimit,
                startingAfter,
                endingBefore,
              ),
            ]);
          }
        } else {
          [res, aggRes] = await (config.aggregatedFetcher
            ? Promise.all([
              config.fetcher(range, ...fetcherArgs),
              config.aggregatedFetcher(range, ...fetcherArgs),
            ])
            : Promise.all([config.fetcher(range, ...fetcherArgs)]));
        }
        if (currentFetchId !== fetchIdRef.current) return;
        let data = res.data || [];
        const chartData = aggRes?.data || data;

        // Calculate pagination cursors from original data before reversing
        const originalData = data;

        const getCursor = (item: Record<string, unknown>) => {
          // For prove-times and verify-times tables, the "name" field is the batch ID
          if (tableType === 'prove-times' || tableType === 'verify-times') {
            const name = (item as { name?: string }).name;
            return name !== undefined ? Number(name) : undefined;
          }
          // Otherwise fall back to the numeric value or block/batch fields
          return (
            (item as { value?: number }).value ??
            (item as { l2_block_number?: number }).l2_block_number ??
            (item as { block?: number }).block ??
            (item as { batch?: number }).batch
          );
        };
        const nextCursor =
          originalData.length > 0 ? getCursor(originalData[originalData.length - 1]) : undefined;
        const prevCursor =
          originalData.length > 0 ? getCursor(originalData[0]) : undefined;

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
        const chart = config.chart ? config.chart(chartData) : undefined;

        if (currentFetchId === fetchIdRef.current) {
          const view: TableViewState = {
            title,
            description: config.description,
            columns: config.columns,
            rows: mappedData,
            chart,
          };
          if (config.supportsPagination) {
            const disablePrev = page === 0;
            const disableNext = originalData.length < PAGE_LIMIT;
            view.serverPagination = {
              page,
              onNext: () => {
                const params = new URLSearchParams(searchParams);
                params.set('page', String(page + 1));
                if (nextCursor !== undefined)
                  params.set('start', String(nextCursor));
                params.delete('end');
                setSearchParams(params);
              },
              onPrev: () => {
                const params = new URLSearchParams(searchParams);
                const newPage = page - 1;
                params.set('page', String(newPage));
                if (newPage === 0) {
                  // On first page, clear all cursor params
                  params.delete('start');
                  params.delete('end');
                } else {
                  if (prevCursor !== undefined) params.set('end', String(prevCursor));
                  params.delete('start');
                }
                setSearchParams(params);
              },
              disableNext,
              disablePrev,
            };
          }
          setTableView(view);
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
