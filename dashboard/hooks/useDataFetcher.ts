import { useCallback, useEffect, useMemo, useState, useRef } from 'react';
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
  const [isTimeRangeChanging, setIsTimeRangeChanging] = useState(false);
  const [lastFetchedTimeRange, setLastFetchedTimeRange] =
    useState<TimeRange | null>(null);

  // Track if component is mounted to prevent state updates after unmount
  const isMountedRef = useRef(true);

  // Use a ref to track the current time range to prevent race conditions
  const currentTimeRangeRef = useRef(timeRange);

  useEffect(() => {
    currentTimeRangeRef.current = timeRange;
  }, [timeRange]);

  useEffect(() => {
    return () => {
      isMountedRef.current = false;
    };
  }, []);

  // Memoize the specific value we need to prevent infinite re-renders
  const viewParam = searchParams.get('view');
  const isTableView = useMemo(
    () => tableView || viewParam === 'table',
    [tableView, viewParam],
  );

  const fetchKey = isTableView
    ? null
    : ['metrics', timeRange, selectedSequencer];

  // Enhanced error handling with mounted check and timeout
  const { data, mutate, isLoading, isValidating } = useSWR(
    fetchKey,
    async () => {
      const currentRange = currentTimeRangeRef.current;
      try {
        const result = await Promise.race([
          fetchMetricsData(currentRange, selectedSequencer),
          new Promise((_, reject) =>
            setTimeout(() => reject(new Error('Request timeout')), 30000)
          )
        ]);
        return result;
      } catch (error) {
        // Always clear loading state on any error
        if (isMountedRef.current) {
          setIsTimeRangeChanging(false);
        }
        throw error;
      }
    },
    {
      refreshInterval: Math.max(refreshRate, 60000),
      revalidateOnFocus: false,
      refreshWhenHidden: false,
      onSuccess: () => {
        // Only update state if component is still mounted and for current timeRange
        if (isMountedRef.current && currentTimeRangeRef.current === timeRange) {
          setIsTimeRangeChanging(false);
          setLastFetchedTimeRange(timeRange);
        }
      },
      onError: () => {
        // Always clear loading state on error if mounted
        if (isMountedRef.current) {
          setIsTimeRangeChanging(false);
          setErrorMessage('Failed to fetch dashboard data. Please try again.');
        }
      },
    },
  );

  // Fix #3: Enhanced time range change detection with proper cleanup
  useEffect(() => {
    if (
      lastFetchedTimeRange &&
      lastFetchedTimeRange !== timeRange &&
      !isTableView &&
      isMountedRef.current
    ) {
      setIsTimeRangeChanging(true);

      // Add a fallback timeout to clear loading state
      const timeoutId = setTimeout(() => {
        if (isMountedRef.current) {
          setIsTimeRangeChanging(false);
          setErrorMessage('Request timed out. Please try again.');
        }
      }, 45000);

      return () => clearTimeout(timeoutId);
    }
  }, [timeRange, lastFetchedTimeRange, isTableView, setErrorMessage]);

  // Clear loading state when switching to table view
  useEffect(() => {
    if (isTableView && isTimeRangeChanging && isMountedRef.current) {
      setIsTimeRangeChanging(false);
    }
  }, [isTableView, isTimeRangeChanging]);

  const fetchData = useCallback(async () => {
    if (!isMountedRef.current) return;

    setIsTimeRangeChanging(true);
    try {
      await mutate();
    } catch (error) {
      if (isMountedRef.current) {
        setIsTimeRangeChanging(false);
        setErrorMessage('Failed to fetch dashboard data. Please try again.');
      }
    }
  }, [mutate, setErrorMessage]);

  const handleManualRefresh = useCallback(() => {
    if (tableView?.onRefresh) {
      tableView.onRefresh();
    } else {
      void fetchData();
    }
  }, [fetchData, tableView?.onRefresh]);

  useEffect(() => {
    if (!data || !isMountedRef.current) return;
    updateLastRefresh();
    if (data.chartData) {
      updateChartsData(data.chartData);
    }
  }, [data, updateChartsData, updateLastRefresh]);

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
