import { describe, it, expect, afterEach, beforeEach, vi } from 'vitest';
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
    json: async () => ({ price }),
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

it('rejects on fetch failure', async () => {
  globalThis.fetch = mockFetch(0, false);
  await expect(getEthPrice()).rejects.toThrowError();
});

it('rejects on network error', async () => {
  globalThis.fetch = mockFetchWithNetworkError();
  await expect(getEthPrice()).rejects.toThrowError();
});

it('rejects on invalid response format', async () => {
  globalThis.fetch = mockFetchWithInvalidResponse();
  await expect(getEthPrice()).rejects.toThrowError();
});
});
