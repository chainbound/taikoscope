import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { getEthPrice } from '../services/priceService.ts';

const originalFetch = globalThis.fetch;

function mockFetch(price: number) {
  return vi.fn(async () => ({
    ok: true,
    json: async () => ({ ethereum: { usd: price } }),
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
});
