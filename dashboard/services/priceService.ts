export const getEthPrice = async (): Promise<number> => {
  const CACHE_KEY = 'ethPrice';
  const oneHour = 3600_000;

  if (typeof localStorage !== 'undefined') {
    const cached = localStorage.getItem(CACHE_KEY);
    if (cached) {
      try {
        const { price, timestamp } = JSON.parse(cached) as {
          price: number;
          timestamp: number;
        };
        if (Date.now() - timestamp < oneHour && typeof price === 'number') {
          return price;
        }
      } catch {
        // ignore malformed cache
      }
    }
  }

  const res = await fetch(
    'https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd',
  );
  const data = (await res.json()) as { ethereum: { usd: number } };
  const price = data.ethereum.usd;

  if (typeof localStorage !== 'undefined') {
    try {
      localStorage.setItem(
        CACHE_KEY,
        JSON.stringify({ price, timestamp: Date.now() }),
      );
    } catch {
      // ignore storage errors
    }
  }

  return price;
};
