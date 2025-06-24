import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import * as swr from 'swr';
vi.mock('swr', () => ({ default: vi.fn() }));
import * as priceService from '../services/priceService';
import { ProfitabilityChart } from '../components/ProfitabilityChart';

// Helper data with negative profit
const feeData = [
  { batch: 1, priority: 0, base: 0, l1Cost: 0 },
];

describe('ProfitabilityChart', () => {
  it('renders when profit is negative', () => {
    vi.mocked(swr.default).mockReturnValue({ data: { data: feeData } } as any);
    vi.spyOn(priceService, 'useEthPrice').mockReturnValue({ data: 1 } as any);

    const html = renderToStaticMarkup(
      React.createElement(ProfitabilityChart, {
        timeRange: '1h',
        cloudCost: 1000,
        proverCost: 1000,
      })
    );

    expect(html).toContain('recharts-responsive-container');
  });
});
