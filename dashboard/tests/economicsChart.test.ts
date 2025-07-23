import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import * as swr from 'swr';
vi.mock('swr', () => ({ default: vi.fn() }));
import type { RequestResult } from '../services/apiService';
import type { BatchFeeComponent } from '../types';
import * as priceService from '../services/priceService';
import { EconomicsChart } from '../components/EconomicsChart';

const data = [{ value: 100, timestamp: 1 }];

describe('EconomicsChart', () => {
  it('renders with economics data', () => {
    vi.mocked(swr.default).mockReturnValue({
      data: { data: data } as unknown as RequestResult<BatchFeeComponent[]>,
    } as unknown as ReturnType<typeof swr.default>);
    vi.spyOn(priceService, 'useEthPrice').mockReturnValue({
      data: 1,
    } as unknown as ReturnType<typeof priceService.useEthPrice>);

    const html = renderToStaticMarkup(
      React.createElement(EconomicsChart, {
        timeRange: '1h',
        cloudCost: 100,
        proverCost: 100,
      }),
    );

    expect(html).toContain('recharts-responsive-container');
  });
});
