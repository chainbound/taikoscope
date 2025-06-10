import { useState, useMemo } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useErrorHandler } from './useErrorHandler';
import type { MetricData } from '../types';

export const useMetricsData = () => {
  const [metrics, setMetrics] = useState<MetricData[]>([]);
  const [loadingMetrics, setLoadingMetrics] = useState(true);
  const { errorMessage, setErrorMessage } = useErrorHandler();

  const [searchParams] = useSearchParams();

  // Memoize the specific value we need to prevent infinite re-renders
  const viewParam = searchParams.get('view');
  const isEconomicsView = useMemo(() => viewParam === 'economics', [viewParam]);

  return useMemo(
    () => ({
      metrics,
      setMetrics,
      loadingMetrics,
      setLoadingMetrics,
      errorMessage,
      setErrorMessage,
      isEconomicsView,
    }),
    [metrics, loadingMetrics, errorMessage, isEconomicsView],
  );
};
