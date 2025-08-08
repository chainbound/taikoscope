import useSWR from 'swr';
import { useEffect, useMemo, useRef, useState } from 'react';
import { API_BASE } from './apiService';
import { showToast } from '../utils/toast';

const CACHE_KEY = 'ethPrice';
const API_URL = `${API_BASE}/eth-price`;

export const getEthPrice = async (): Promise<number> => {
  let res: Response;
  try {
    res = await fetch(API_URL);
  } catch (_err) {
    throw new Error('Failed to fetch ETH price');
  }
  if (!res.ok) {
    throw new Error('Failed to fetch ETH price');
  }

  try {
    const data = await res.json();
    const price = data?.price;
    if (typeof price !== 'number' || !isFinite(price) || price <= 0) {
      throw new Error('Failed to fetch ETH price');
    }
    return price;
  } catch (_err) {
    throw new Error('Failed to fetch ETH price');
  }
};

export const useEthPrice = () => {
  const [lastUpdatedAt, setLastUpdatedAt] = useState<number | undefined>(
    undefined,
  );

  // Read any cached price regardless of age; keep timestamp for staleness signal
  const cached = useMemo(() => {
    if (typeof localStorage === 'undefined') return undefined as
      | { price: number; timestamp: number }
      | undefined;
    const raw = localStorage.getItem(CACHE_KEY);
    if (!raw) return undefined;
    try {
      const parsed = JSON.parse(raw) as { price: number; timestamp: number };
      if (typeof parsed.price === 'number' && isFinite(parsed.price)) {
        return parsed;
      }
    } catch {
      // ignore malformed cache
    }
    return undefined;
  }, []);

  // Initialize lastUpdatedAt from cache if present
  useEffect(() => {
    if (cached?.timestamp) {
      setLastUpdatedAt(cached.timestamp);
    }
  }, [cached?.timestamp]);

  const swr = useSWR<number>('ethPrice', getEthPrice, {
    revalidateOnFocus: false,
    refreshInterval: 60_000,
    errorRetryCount: Infinity,
    errorRetryInterval: 15_000,
    fallbackData: cached?.price,
    dedupingInterval: 10_000,
  });

  // Persist successful updates and bump lastUpdatedAt
  useEffect(() => {
    if (typeof localStorage === 'undefined') return;
    if (typeof swr.data === 'number' && isFinite(swr.data) && swr.data > 0) {
      const now = Date.now();
      try {
        localStorage.setItem(
          CACHE_KEY,
          JSON.stringify({ price: swr.data, timestamp: now }),
        );
      } catch {
        // ignore storage errors
      }
      setLastUpdatedAt(now);
    }
  }, [swr.data]);

  // Only raise error if there is no cached value and no current data
  const hasCached = cached?.price != null;
  const error = !hasCached && swr.data == null ? swr.error : undefined;

  // Optional toast on first cold-start failure
  const hasToastedRef = useRef(false);
  useEffect(() => {
    if (!hasToastedRef.current && error) {
      showToast('Failed to fetch ETH price');
      hasToastedRef.current = true;
    }
  }, [error]);

  const isStale = useMemo(() => {
    if (!lastUpdatedAt) return undefined;
    return Date.now() - lastUpdatedAt > 3_600_000; // 1 hour
  }, [lastUpdatedAt]);

  return { ...swr, error, lastUpdatedAt, isStale } as typeof swr & {
    error?: Error;
    lastUpdatedAt?: number;
    isStale?: boolean;
  };
};
