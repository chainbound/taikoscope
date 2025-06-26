import { describe, it, expect } from 'vitest';
import { SmartCache } from '../utils/smartCache';

function wait(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

describe('SmartCache', () => {
  it('stores and retrieves values', () => {
    const cache = new SmartCache<number>(2, 100);
    cache.set('a', 1);
    expect(cache.get('a')).toBe(1);
    const stats = cache.getStats();
    expect(stats.hits).toBe(1);
    expect(stats.misses).toBe(0);
  });

  it('expires entries based on ttl', async () => {
    const cache = new SmartCache<number>(2, 5);
    cache.set('b', 2);
    await wait(10);
    expect(cache.get('b')).toBeNull();
    const stats = cache.getStats();
    expect(stats.misses).toBe(1);
  });

  it('evicts least recently used item', async () => {
    const cache = new SmartCache<number>(2, 1000);
    cache.set('a', 1);
    await wait(5);
    cache.set('b', 2);
    await wait(5);
    // Access 'a' so 'b' becomes least recently used
    cache.get('a');
    await wait(5);
    cache.set('c', 3);
    expect(cache.has('b')).toBe(false);
    expect(cache.has('a')).toBe(true);
    expect(cache.has('c')).toBe(true);
  });

  it('invalidates keys by prefix', () => {
    const cache = new SmartCache<number>(2, 1000);
    cache.set('prefix_one', 1);
    cache.set('prefix_two', 2);
    cache.invalidate('prefix');
    expect(cache.has('prefix_one')).toBe(false);
    expect(cache.has('prefix_two')).toBe(false);
  });

  it('getOrSet caches factory result', async () => {
    const cache = new SmartCache<number>(2, 1000);
    const value = await cache.getOrSet('d', async () => 4);
    expect(value).toBe(4);
    expect(cache.get('d')).toBe(4);
  });
});
