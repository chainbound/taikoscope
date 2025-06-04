import { useCallback, useEffect, useMemo } from 'react';
import useSWR from 'swr';
import { TimeRange } from '../types';
import { useSearchParams } from './useSearchParams';
import { TableViewState } from './useTableActions';

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
  const searchParams = useSearchParams();

  // Memoize the specific value we need to prevent infinite re-renders
  const viewParam = searchParams.get('view');
  const isTableView = useMemo(
    () => tableView || viewParam === 'table',
    [tableView, viewParam],
  );

  const fetchKey = isTableView
    ? null
    : ['metrics', timeRange, selectedSequencer];

  const { data, mutate } = useSWR(
    fetchKey,
    () => fetchMetricsData(timeRange, selectedSequencer),
    {
      refreshInterval: Math.max(refreshRate, 60000),
      revalidateOnFocus: true,
      refreshWhenHidden: false,
    },
  );

  const fetchData = useCallback(async () => {
    await mutate();
  }, [mutate]);

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

  return {
    fetchData,
    handleManualRefresh,
  };
};
