import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { ProfitCalculator } from '../components/ProfitCalculator';
import * as priceService from '../services/priceService';

describe('ProfitCalculator', () => {
  it('calculates profit for time range', () => {
    vi.spyOn(priceService, 'useEthPrice').mockReturnValue({
      data: 2000,
    } as any);
    const html = renderToStaticMarkup(
      React.createElement(ProfitCalculator, {
        metrics: [
          { title: 'Priority Fee', value: '0.6 ETH' },
          { title: 'Base Fee', value: '0.4 ETH' },
          { title: 'L1 Data Cost', value: '0.1 ETH' },
        ],
        timeRange: '1h',
        cloudCost: 100,
        proverCost: 100,
        onCloudCostChange: () => {},
        onProverCostChange: () => {},
      }),
    );
    expect(html.includes('1,799')).toBe(true);
  });

  it('rejects negative values via min attribute', () => {
    vi.spyOn(priceService, 'useEthPrice').mockReturnValue({
      data: 2000,
    } as any);
    const html = renderToStaticMarkup(
      React.createElement(ProfitCalculator, {
        metrics: [
          { title: 'Priority Fee', value: '0.6 ETH' },
          { title: 'Base Fee', value: '0.4 ETH' },
        ],
        timeRange: '1h',
        cloudCost: 0,
        proverCost: 0,
        onCloudCostChange: () => {},
        onProverCostChange: () => {},
      }),
    );
    const matches = html.match(/min="0"/g) ?? [];
    expect(matches.length).toBe(2);
  });
});
