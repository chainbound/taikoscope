import { useCallback, useEffect, useState } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { TimeRange } from '../types';
import { isValidTimeRange } from '../utils/timeRange';

const DEFAULT_TIME_RANGE: TimeRange = '1h';

/**
 * Hook that synchronizes time range state with URL parameters to prevent navigation loops
 * and maintain consistent state across dashboard and table views.
 */
export const useTimeRangeSync = () => {
  const location = useLocation();
  const navigate = useNavigate();

  // Get initial time range from URL or use default
  const getInitialTimeRange = useCallback((): TimeRange => {
    const params = new URLSearchParams(location.search);
    const urlRange = params.get('range');
    return urlRange && isValidTimeRange(urlRange)
      ? (urlRange as TimeRange)
      : DEFAULT_TIME_RANGE;
  }, [location.search]);

  const [timeRange, setTimeRangeState] =
    useState<TimeRange>(getInitialTimeRange);

  // Update time range and sync with URL
  const setTimeRange = useCallback(
    (newRange: TimeRange) => {
      if (!isValidTimeRange(newRange)) {
        console.warn('Invalid time range:', newRange);
        return;
      }

      setTimeRangeState(newRange);

      // Update URL parameters without affecting navigation
      const newParams = new URLSearchParams(location.search);
      if (newRange === DEFAULT_TIME_RANGE) {
        newParams.delete('range');
      } else {
        newParams.set('range', newRange);
      }

      // Use replace to avoid adding history entries for time range changes
      // Only update query parameters without forcing navigation to '/'
      navigate(
        { search: newParams.toString() },
        { replace: true },
      );
    },
    [location.search, navigate],
  );

  // Sync state when URL changes (e.g., browser back/forward)
  useEffect(() => {
    const urlRange = getInitialTimeRange();
    if (urlRange !== timeRange) {
      setTimeRangeState(urlRange);
    }
  }, [location.search, timeRange, getInitialTimeRange]);

  return {
    timeRange,
    setTimeRange,
  };
};
