import { describe, it, expect } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { BlockTimeDistributionChart } from '../components/BlockTimeDistributionChart';
import type { TimeSeriesData } from '../types';

const MIN_BIN_COUNT = 5;
const MAX_BIN_COUNT = 20;
const MIN_MS = 0;
const MAX_MS = 24 * 60 * 60 * 1000;

function computeDistribution(data: TimeSeriesData[]) {
  const times = data
    .map((d) => d.timestamp)
    .filter((t) => t >= MIN_MS && t <= MAX_MS);
  if (times.length === 0) return [];
  const min = Math.min(...times);
  const max = Math.max(...times);
  if (min === max) return [{ interval: min, count: times.length }];
  const binCount = Math.min(
    MAX_BIN_COUNT,
    Math.max(MIN_BIN_COUNT, Math.floor(Math.sqrt(times.length))),
  );
  const binSize = (max - min) / binCount;
  const EPSILON = 1e-10;
  if (binSize < EPSILON) return [{ interval: (min + max) / 2, count: times.length }];
  const bins = Array.from({ length: binCount }, (_, i) => ({
    interval: min + (i + 0.5) * binSize,
    count: 0,
  }));
  times.forEach((t) => {
    const idx = Math.min(Math.floor((t - min) / binSize), binCount - 1);
    bins[idx].count += 1;
  });
  return bins;
}

describe('BlockTimeDistributionChart', () => {
  it('adapts bin count and filters out-of-range values', () => {
    const data: TimeSeriesData[] = [
      { timestamp: -1, value: 0 },
      { timestamp: 0, value: 1 },
      { timestamp: 1000, value: 2 },
      { timestamp: 2000, value: 3 },
      { timestamp: 3000, value: 4 },
      { timestamp: 4000, value: 5 },
      { timestamp: 5000, value: 6 },
      { timestamp: 6000, value: 7 },
      { timestamp: 7000, value: 8 },
      { timestamp: 8000, value: 9 },
      { timestamp: 9000, value: 10 },
      { timestamp: MAX_MS + 1, value: 11 },
    ];
    const html = renderToStaticMarkup(
      React.createElement(BlockTimeDistributionChart, { data, barColor: '#000' }),
    );
    expect(html).toContain('recharts-responsive-container');

    const dist = computeDistribution(data);
    expect(dist.length).toBe(5);
    const total = dist.reduce((sum, b) => sum + b.count, 0);
    expect(total).toBe(10);
  });

  it('caps bin count at the maximum', () => {
    const bigData: TimeSeriesData[] = Array.from({ length: 500 }, (_, i) => ({
      timestamp: i * 100,
      value: i,
    }));
    renderToStaticMarkup(
      React.createElement(BlockTimeDistributionChart, {
        data: bigData,
        barColor: '#000',
      }),
    );
    const dist = computeDistribution(bigData);
    expect(dist.length).toBe(MAX_BIN_COUNT);
  });
});
