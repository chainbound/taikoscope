import useSWR from 'swr';
import { useEffect } from 'react';
import { API_BASE } from './apiService';
import { showToast } from '../utils/toast';

const CACHE_KEY = 'ethPrice';
const API_URL = `${API_BASE}/eth-price`;

export const getEthPrice = async (): Promise<number> => {
  try {
    const res = await fetch(API_URL);
    if (!res.ok) {
      throw new Error(res.statusText);
    }

    const data = await res.json();
    const price = data?.price;
    if (typeof price !== 'number') {
      throw new Error('invalid response');
    }
    return price;
  } catch (e) {
    showToast('Failed to fetch ETH price');
    throw e instanceof Error ? e : new Error('Failed to fetch ETH price');
  }
};

export const useEthPrice = () => {
  const fallbackData =
    typeof localStorage === 'undefined'
      ? undefined
      : (() => {
          const cached = localStorage.getItem(CACHE_KEY);
          if (cached) {
            try {
              const { price, timestamp } = JSON.parse(cached) as {
                price: number;
                timestamp: number;
              };
              if (
                Date.now() - timestamp < 3600_000 &&
                typeof price === 'number'
              ) {
                return price;
              }
            } catch {
              // ignore malformed cache
            }
          }
          return undefined;
        })();

  const swr = useSWR<number>('ethPrice', getEthPrice, {
    revalidateOnFocus: false,
    fallbackData,
  });

  useEffect(() => {
    if (
      typeof localStorage !== 'undefined' &&
      swr.data !== undefined &&
      swr.data !== 0
    ) {
      try {
        localStorage.setItem(
          CACHE_KEY,
          JSON.stringify({ price: swr.data, timestamp: Date.now() }),
        );
      } catch {
        // ignore storage errors
      }
    }
  }, [swr.data]);

  const error = swr.data === undefined ? swr.error : undefined;

  return { ...swr, error };
};
