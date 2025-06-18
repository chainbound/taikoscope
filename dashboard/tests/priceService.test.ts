import { describe, it, expect, afterEach, beforeEach, vi } from 'vitest';
import * as toast from '../utils/toast';
import { getEthPrice } from '../services/priceService.ts';

const originalFetch = globalThis.fetch;

beforeEach(() => {
  vi.stubGlobal('window', {
    dispatchEvent: vi.fn(),
  });
});

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

function mockFetchWithNetworkError() {
  return vi.fn(async () => {
    throw new Error('network error');
  }) as unknown as typeof fetch;
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

  it('handles fetch failure', async () => {
    globalThis.fetch = mockFetch(0, false);
    const spy = vi.spyOn(toast, 'showToast').mockImplementation(() => {});
    await expect(getEthPrice()).rejects.toThrow(
      'Failed to fetch ETH price: 500',
    );
    expect(spy).toHaveBeenCalled();
    spy.mockRestore();
  });

  it('returns 0 on network error', async () => {
    globalThis.fetch = mockFetchWithNetworkError();
    const spy = vi.spyOn(toast, 'showToast').mockImplementation(() => {});
    const price = await getEthPrice();
    expect(price).toBe(0);
    expect(spy).toHaveBeenCalled();
    spy.mockRestore();
  });

  it('handles invalid response format', async () => {
    globalThis.fetch = mockFetchWithInvalidResponse();
    await expect(getEthPrice()).rejects.toThrow(
      'Invalid ETH price response format',
    );
  });
});
