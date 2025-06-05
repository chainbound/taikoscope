import { useState, useEffect, useCallback, useMemo } from 'react';
import { loadRefreshRate, saveRefreshRate, isValidRefreshRate } from '../utils';

export const useRefreshTimer = () => {
  const [refreshRate, setRefreshRateState] = useState<number>(() =>
    loadRefreshRate(),
  );
  const [lastRefresh, setLastRefresh] = useState<number>(Date.now());

  const setRefreshRate = useCallback((rate: number) => {
    if (isValidRefreshRate(rate)) {
      setRefreshRateState(rate);
    }
  }, []);

  useEffect(() => {
    saveRefreshRate(refreshRate);
  }, [refreshRate]);

  const updateLastRefresh = useCallback(() => {
    setLastRefresh(Date.now());
  }, []);

  return useMemo(
    () => ({
      refreshRate,
      setRefreshRate,
      lastRefresh,
      updateLastRefresh,
    }),
    [refreshRate, setRefreshRate, lastRefresh, updateLastRefresh],
  );
};
