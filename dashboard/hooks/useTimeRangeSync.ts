import { useCallback, useEffect, useState, useMemo } from 'react';
import { useSearchParams, useNavigate } from 'react-router-dom';
import { TimeRange } from '../types';

const DEFAULT_TIME_RANGE: TimeRange = '1h';
const VALID_TIME_RANGES: TimeRange[] = ['1h', '24h', '7d'];

/**
 * Hook that synchronizes time range state with URL parameters to prevent navigation loops
 * and maintain consistent state across dashboard and table views.
 */
export const useTimeRangeSync = () => {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();

  // Get initial time range from URL or use default
  const getInitialTimeRange = useCallback((): TimeRange => {
    const urlRange = searchParams.get('range') as TimeRange;
    return urlRange && VALID_TIME_RANGES.includes(urlRange)
      ? urlRange
      : DEFAULT_TIME_RANGE;
  }, [searchParams]);

  const [timeRange, setTimeRangeState] =
    useState<TimeRange>(getInitialTimeRange);

  // Update time range and sync with URL
  const setTimeRange = useCallback(
    (newRange: TimeRange) => {
      if (!VALID_TIME_RANGES.includes(newRange)) {
        console.warn('Invalid time range:', newRange);
        return;
      }

      setTimeRangeState(newRange);

      // Update URL parameters without affecting navigation
      const newParams = new URLSearchParams(searchParams);
      if (newRange === DEFAULT_TIME_RANGE) {
        newParams.delete('range');
      } else {
        newParams.set('range', newRange);
      }

      // Use replace to avoid adding history entries for time range changes
      navigate(
        { pathname: '/', search: newParams.toString() },
        { replace: true },
      );
    },
    [searchParams, navigate],
  );

  // Sync state when URL changes (e.g., browser back/forward)
  useEffect(() => {
    const urlRange = getInitialTimeRange();
    if (urlRange !== timeRange) {
      setTimeRangeState(urlRange);
    }
  }, [searchParams, timeRange, getInitialTimeRange]);

  return useMemo(
    () => ({
      timeRange,
      setTimeRange,
    }),
    [timeRange, setTimeRange],
  );
};
