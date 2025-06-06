export interface CacheEntry<T> {
  data: T;
  timestamp: number;
  ttl: number; // Time to live in milliseconds
  accessCount: number;
  lastAccessed: number;
}

export interface CacheStats {
  hits: number;
  misses: number;
  evictions: number;
  totalEntries: number;
  hitRate: number;
}

export class SmartCache<T> {
  private cache: Map<string, CacheEntry<T>>;
  private maxSize: number;
  private defaultTTL: number;
  private stats: CacheStats;

  constructor(maxSize: number = 100, defaultTTL: number = 5 * 60 * 1000) {
    this.cache = new Map();
    this.maxSize = maxSize;
    this.defaultTTL = defaultTTL;
    this.stats = {
      hits: 0,
      misses: 0,
      evictions: 0,
      totalEntries: 0,
      hitRate: 0,
    };
  }

  /**
   * Generate a cache key from parameters
   */
  private generateKey(baseKey: string, params?: Record<string, any>): string {
    if (!params) return baseKey;

    const sortedParams = Object.keys(params)
      .sort()
      .map((key) => `${key}:${JSON.stringify(params[key])}`)
      .join('|');

    return `${baseKey}?${sortedParams}`;
  }

  /**
   * Check if an entry is expired
   */
  private isExpired(entry: CacheEntry<T>): boolean {
    return Date.now() - entry.timestamp > entry.ttl;
  }

  /**
   * Update cache statistics
   */
  private updateStats() {
    const total = this.stats.hits + this.stats.misses;
    this.stats.hitRate = total > 0 ? this.stats.hits / total : 0;
    this.stats.totalEntries = this.cache.size;
  }

  /**
   * Evict least recently used entries when cache is full
   */
  private evictLRU() {
    if (this.cache.size < this.maxSize) return;

    let oldestKey = '';
    let oldestAccessed = Date.now();

    for (const [key, entry] of this.cache.entries()) {
      if (entry.lastAccessed < oldestAccessed) {
        oldestAccessed = entry.lastAccessed;
        oldestKey = key;
      }
    }

    if (oldestKey) {
      this.cache.delete(oldestKey);
      this.stats.evictions++;
    }
  }

  /**
   * Clean up expired entries
   */
  private cleanup() {
    const expiredKeys: string[] = [];

    for (const [key, entry] of this.cache.entries()) {
      if (this.isExpired(entry)) {
        expiredKeys.push(key);
      }
    }

    expiredKeys.forEach((key) => {
      this.cache.delete(key);
      this.stats.evictions++;
    });
  }

  /**
   * Get cached data
   */
  get(key: string, params?: Record<string, any>): T | null {
    const cacheKey = this.generateKey(key, params);
    const entry = this.cache.get(cacheKey);

    if (!entry) {
      this.stats.misses++;
      this.updateStats();
      return null;
    }

    if (this.isExpired(entry)) {
      this.cache.delete(cacheKey);
      this.stats.misses++;
      this.stats.evictions++;
      this.updateStats();
      return null;
    }

    // Update access tracking
    entry.accessCount++;
    entry.lastAccessed = Date.now();
    this.stats.hits++;
    this.updateStats();

    return entry.data;
  }

  /**
   * Set cached data
   */
  set(key: string, data: T, params?: Record<string, any>, ttl?: number): void {
    const cacheKey = this.generateKey(key, params);
    const now = Date.now();

    // Clean up before adding new entries
    this.cleanup();
    this.evictLRU();

    const entry: CacheEntry<T> = {
      data,
      timestamp: now,
      ttl: ttl || this.defaultTTL,
      accessCount: 1,
      lastAccessed: now,
    };

    this.cache.set(cacheKey, entry);
    this.updateStats();
  }

  /**
   * Check if data exists and is not expired
   */
  has(key: string, params?: Record<string, any>): boolean {
    const cacheKey = this.generateKey(key, params);
    const entry = this.cache.get(cacheKey);

    if (!entry) return false;
    if (this.isExpired(entry)) {
      this.cache.delete(cacheKey);
      this.stats.evictions++;
      this.updateStats();
      return false;
    }

    return true;
  }

  /**
   * Invalidate specific cache entry or pattern
   */
  invalidate(key: string, params?: Record<string, any>): void {
    if (params) {
      const cacheKey = this.generateKey(key, params);
      this.cache.delete(cacheKey);
    } else {
      // Invalidate all entries that start with the key
      const keysToDelete: string[] = [];
      for (const cacheKey of this.cache.keys()) {
        if (cacheKey.startsWith(key)) {
          keysToDelete.push(cacheKey);
        }
      }
      keysToDelete.forEach((k) => this.cache.delete(k));
      this.stats.evictions += keysToDelete.length;
    }
    this.updateStats();
  }

  /**
   * Clear all cache entries
   */
  clear(): void {
    this.stats.evictions += this.cache.size;
    this.cache.clear();
    this.updateStats();
  }

  /**
   * Get cache statistics
   */
  getStats(): CacheStats {
    this.updateStats();
    return { ...this.stats };
  }

  /**
   * Get cache entries for debugging
   */
  getEntries(): Array<{ key: string; entry: CacheEntry<T> }> {
    return Array.from(this.cache.entries()).map(([key, entry]) => ({
      key,
      entry: { ...entry },
    }));
  }

  /**
   * Get or set with a factory function
   */
  async getOrSet(
    key: string,
    factory: () => Promise<T>,
    params?: Record<string, any>,
    ttl?: number,
  ): Promise<T> {
    const cached = this.get(key, params);
    if (cached !== null) {
      return cached;
    }

    const data = await factory();
    this.set(key, data, params, ttl);
    return data;
  }
}

// Global cache instances for different data types
export const dashboardCache = new SmartCache<any>(200, 5 * 60 * 1000); // 5 minutes TTL
export const chartCache = new SmartCache<any>(100, 3 * 60 * 1000); // 3 minutes TTL
export const tableCache = new SmartCache<any>(50, 10 * 60 * 1000); // 10 minutes TTL

// Cache management utilities
export const cacheManager = {
  invalidateAll: () => {
    dashboardCache.clear();
    chartCache.clear();
    tableCache.clear();
  },

  invalidateTimeRange: (timeRange: string) => {
    dashboardCache.invalidate('metrics', { timeRange });
    chartCache.invalidate('chart', { timeRange });
    tableCache.invalidate('table', { timeRange });
  },

  getOverallStats: () => {
    const dashboard = dashboardCache.getStats();
    const chart = chartCache.getStats();
    const table = tableCache.getStats();

    return {
      dashboard,
      chart,
      table,
      combined: {
        hits: dashboard.hits + chart.hits + table.hits,
        misses: dashboard.misses + chart.misses + table.misses,
        evictions: dashboard.evictions + chart.evictions + table.evictions,
        totalEntries:
          dashboard.totalEntries + chart.totalEntries + table.totalEntries,
        hitRate:
          (dashboard.hits + chart.hits + table.hits) /
            (dashboard.hits +
              chart.hits +
              table.hits +
              dashboard.misses +
              chart.misses +
              table.misses) || 0,
      },
    };
  },
};
