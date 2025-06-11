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
        ],
        timeRange: '1h',
      }),
    );
    expect(html.includes('1999.72')).toBe(true);
  });
});
