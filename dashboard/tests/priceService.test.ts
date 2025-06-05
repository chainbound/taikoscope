import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { getEthPrice } from '../services/priceService.ts';

const originalFetch = globalThis.fetch;

function mockFetch(price: number, ok = true) {
  return vi.fn(async () => ({
    ok,
    status: ok ? 200 : 500,
    json: async () => ({ ethereum: { usd: price } }),
  })) as unknown as typeof fetch;
}

function mockFetchWithInvalidResponse() {
  return vi.fn(async () => ({
    ok: true,
    json: async () => ({ invalid: 'response' }),
  })) as unknown as typeof fetch;
}

const store: Record<string, string> = {};

beforeEach(() => {
  globalThis.localStorage = {
    getItem: (k: string) => (k in store ? store[k] : null),
    setItem: (k: string, v: string) => {
      store[k] = v;
    },
    removeItem: (k: string) => {
      delete store[k];
    },
    clear: () => {
      for (const k in store) delete store[k];
    },
    key: () => null,
    length: 0,
  } as Storage;
});

afterEach(() => {
  globalThis.fetch = originalFetch;
  for (const k in store) delete store[k];
});

describe('getEthPrice', () => {
  it('caches price for one hour', async () => {
    globalThis.fetch = mockFetch(2000);
    const first = await getEthPrice();
    expect(first).toBe(2000);
    expect(globalThis.fetch).toHaveBeenCalledTimes(1);

    const second = await getEthPrice();
    expect(second).toBe(2000);
    expect(globalThis.fetch).toHaveBeenCalledTimes(1);
  });

  it('refreshes after cache expiry', async () => {
    store.ethPrice = JSON.stringify({ price: 1500, timestamp: Date.now() - 3600_001 });
    globalThis.fetch = mockFetch(1800);
    const price = await getEthPrice();
    expect(price).toBe(1800);
  });

  it('handles fetch failure', async () => {
    globalThis.fetch = mockFetch(0, false);
    await expect(getEthPrice()).rejects.toThrow('Failed to fetch ETH price: 500');
  });

  it('handles invalid response format', async () => {
    globalThis.fetch = mockFetchWithInvalidResponse();
    await expect(getEthPrice()).rejects.toThrow('Invalid ETH price response format');
  });

  it('handles malformed cache data', async () => {
    store.ethPrice = 'invalid json';
    globalThis.fetch = mockFetch(2500);
    const price = await getEthPrice();
    expect(price).toBe(2500);
    expect(globalThis.fetch).toHaveBeenCalledTimes(1);
  });

  it('handles non-numeric price in cache', async () => {
    store.ethPrice = JSON.stringify({ price: 'invalid', timestamp: Date.now() });
    globalThis.fetch = mockFetch(3000);
    const price = await getEthPrice();
    expect(price).toBe(3000);
    expect(globalThis.fetch).toHaveBeenCalledTimes(1);
  });
});
