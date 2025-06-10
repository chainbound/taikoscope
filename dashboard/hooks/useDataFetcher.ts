import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import useSWR from 'swr';
import { TimeRange } from '../types';
import { useSearchParams } from 'react-router-dom';
import { TableViewState } from './useTableActions';

interface UseDataFetcherProps {
  timeRange: TimeRange;
  selectedSequencer: string | null;
  tableView: TableViewState | null;
  fetchMetricsData: (
    timeRange: TimeRange,
    selectedSequencer: string | null,
    signal?: AbortSignal,
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
  const [isTimeRangeChanging, setIsTimeRangeChanging] = useState(false);
  const [lastFetchedTimeRange, setLastFetchedTimeRange] =
    useState<TimeRange | null>(null);
  const abortRef = useRef<AbortController | null>(null);
  const requestRangeRef = useRef<TimeRange>(timeRange);

  // Memoize the specific value we need to prevent infinite re-renders
  const viewParam = searchParams.get('view');
  const isTableView = useMemo(
    () => tableView || viewParam === 'table',
    [tableView, viewParam],
  );

  const fetchKey = isTableView
    ? null
    : ['metrics', timeRange, selectedSequencer];

  useEffect(() => {
    abortRef.current?.abort();
  }, [timeRange]);

  useEffect(() => {
    return () => {
      abortRef.current?.abort();
    };
  }, []);

  const fetcher = () => {
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;
    requestRangeRef.current = timeRange;
    return fetchMetricsData(timeRange, selectedSequencer, controller.signal);
  };

  const { data, mutate, isLoading, isValidating } = useSWR(
    fetchKey,
    fetcher,
    {
      refreshInterval: Math.max(refreshRate, 60000),
      revalidateOnFocus: false,
      refreshWhenHidden: false,
      onSuccess: () => {
        if (requestRangeRef.current === timeRange) {
          setIsTimeRangeChanging(false);
          setLastFetchedTimeRange(timeRange);
        }
      },
      onError: () => {
        if (requestRangeRef.current === timeRange) {
          setIsTimeRangeChanging(false);
        }
      },
    },
  );

  // Detect time range changes
  useEffect(() => {
    if (
      lastFetchedTimeRange &&
      lastFetchedTimeRange !== timeRange &&
      !isTableView
    ) {
      setIsTimeRangeChanging(true);
    }
  }, [timeRange, lastFetchedTimeRange, isTableView]);

  const fetchData = useCallback(async () => {
    setIsTimeRangeChanging(true);
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
    if (lastFetchedTimeRange !== timeRange) return;
    updateLastRefresh();
    if (data.chartData) {
      updateChartsData(data.chartData);
    }
  }, [data, updateChartsData, updateLastRefresh, timeRange, lastFetchedTimeRange]);

  // Enhanced loading state that considers both SWR loading and time range changes
  const isLoadingData = isLoading || isValidating || isTimeRangeChanging;

  return {
    fetchData,
    handleManualRefresh,
    isLoadingData,
    isTimeRangeChanging,
    hasData: !!data,
  };
};
