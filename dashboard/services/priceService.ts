import useSWR from 'swr';
import { useEffect } from 'react';

const CACHE_KEY = 'ethPrice';
const API_URL =
  'https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd';

export const getEthPrice = async (): Promise<number> => {
  try {
    const res = await fetch(API_URL);
    if (!res.ok) {
      return 0;
    }

    const data = await res.json();
    const price = data?.ethereum?.usd;
    return typeof price === 'number' ? price : 0;
  } catch {
    return 0;
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

  return swr;
};
