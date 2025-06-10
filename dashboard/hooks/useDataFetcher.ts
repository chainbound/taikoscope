import { useCallback, useEffect, useMemo } from 'react';
import useSWR from 'swr';
import { TimeRange } from '../types';
import { useSearchParams } from 'react-router-dom';
import { TableViewState } from './useTableActions';
import { useErrorHandler } from './useErrorHandler';

interface UseDataFetcherProps {
  timeRange: TimeRange;
  selectedSequencer: string | null;
  tableView: TableViewState | null;
  fetchMetricsData: (
    timeRange: TimeRange,
    selectedSequencer: string | null,
  ) => Promise<any>;
  updateChartsData: (data: any) => void;
  refreshRate: number;
  updateLastRefresh: () => void;
}

export const useDataFetcher = ({
  timeRange,
  selectedSequencer,
  tableView,
  fetchMetricsData,
  updateChartsData,
  refreshRate,
  updateLastRefresh,
}: UseDataFetcherProps) => {
  const [searchParams] = useSearchParams();
  const { setErrorMessage } = useErrorHandler();

  // Memoize the specific value we need to prevent infinite re-renders
  const viewParam = searchParams.get('view');
  const isTableView = useMemo(
    () => tableView || viewParam === 'table',
    [tableView, viewParam],
  );

  const fetchKey = isTableView
    ? null
    : ['metrics', timeRange, selectedSequencer];

  const { data, mutate, isLoading, isValidating } = useSWR(
    fetchKey,
    () => fetchMetricsData(timeRange, selectedSequencer),
    {
      refreshInterval: Math.max(refreshRate, 60000),
      revalidateOnFocus: false,
      refreshWhenHidden: false,
      onError: () => {
        setErrorMessage('Failed to fetch dashboard data. Please try again.');
      },
    },
  );

  const fetchData = useCallback(async () => {
    try {
      await mutate();
    } catch {
      setErrorMessage('Failed to fetch dashboard data. Please try again.');
    }
  }, [mutate, setErrorMessage]);

  const handleManualRefresh = useCallback(() => {
    if (tableView && tableView.onRefresh) {
      // If we're in a table view and it has a refresh function, use that
      tableView.onRefresh();
    } else {
      // Otherwise refresh the main dashboard data
      void fetchData();
    }
  }, [fetchData, tableView?.onRefresh]);

  useEffect(() => {
    if (!data) return;
    updateLastRefresh();
    if (data.chartData) {
      updateChartsData(data.chartData);
    }
  }, [data, updateChartsData, updateLastRefresh]);

  // Enhanced loading state that considers both SWR loading and time range changes
  const isLoadingData = isLoading || isValidating;

  return {
    fetchData,
    handleManualRefresh,
    isLoadingData,
    isTimeRangeChanging: isValidating,
    hasData: !!data,
  };
};
