import { useState, useMemo } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useErrorHandler } from './useErrorHandler';
import type { MetricData, MetricsDataState, DashboardViewType } from '../types';

export const useMetricsData = (): MetricsDataState => {
  const [metrics, setMetrics] = useState<MetricData[]>([]);
  const [loadingMetrics, setLoadingMetrics] = useState(true);
  const { errorMessage, setErrorMessage } = useErrorHandler();

  const [searchParams] = useSearchParams();

  // Memoize the specific value we need to prevent infinite re-renders
  const viewParam = searchParams.get('view') as DashboardViewType | null;
  const view = useMemo<DashboardViewType>(
    () =>
      viewParam === 'performance' ||
      viewParam === 'health' ||
      viewParam === 'economics'
        ? viewParam
        : 'economics',
    [viewParam],
  );

  return useMemo(
    () => ({
      metrics,
      setMetrics,
      loadingMetrics,
      setLoadingMetrics,
      errorMessage,
      setErrorMessage,
      view,
    }),
    [metrics, loadingMetrics, errorMessage, view],
  );
};
