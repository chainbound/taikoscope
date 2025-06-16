import { describe, it, expect, afterEach, vi } from 'vitest';
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

afterEach(() => {
  globalThis.fetch = originalFetch;
});

describe('getEthPrice', () => {
  it('fetches price successfully', async () => {
    globalThis.fetch = mockFetch(2000);
    const price = await getEthPrice();
    expect(price).toBe(2000);
  });

  it('returns 0 on fetch failure', async () => {
    globalThis.fetch = mockFetch(0, false);
    const price = await getEthPrice();
    expect(price).toBe(0);
  });

  it('returns 0 for invalid response format', async () => {
    globalThis.fetch = mockFetchWithInvalidResponse();
    const price = await getEthPrice();
    expect(price).toBe(0);
  });
});
