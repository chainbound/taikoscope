import useSWR from 'swr';
import { useEffect } from 'react';
import { showToast } from '../utils/toast';

const CACHE_KEY = 'ethPrice';
const API_URL =
  'https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd';

export const getEthPrice = async (): Promise<number> => {
  let res: Response;
  try {
    res = await fetch(API_URL);
  } catch {
    showToast('Failed to fetch ETH price');
    return 0;
  }
  if (!res.ok) {
    showToast('Failed to fetch ETH price');
    throw new Error(`Failed to fetch ETH price: ${res.status}`);
  }

  const data = await res.json();
  const price = data?.ethereum?.usd;
  if (typeof price !== 'number') {
    showToast('Failed to fetch ETH price');
    throw new Error('Invalid ETH price response format');
  }

  return price;
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
    if (typeof localStorage !== 'undefined' && swr.data !== undefined) {
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

  const error =
    swr.error ??
    (swr.data === 0 ? new Error('ETH price unavailable') : undefined);

  return { ...swr, error };
};
