import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import * as swr from 'swr';
vi.mock('swr', () => ({ default: vi.fn() }));
import type { RequestResult } from '../services/apiService';
import type { BatchFeeComponent } from '../types';
import * as priceService from '../services/priceService';
import { IncomeChart } from '../components/IncomeChart';

const feeData = [
  {
    batch: 1,
    priority: 1,
    base: 1,
    l1Cost: 0,
    amortizedProveCost: 0,
    amortizedVerifyCost: 0,
  },
];

describe('IncomeChart', () => {
  it('renders with income data', () => {
    vi.mocked(swr.default).mockReturnValue({
      data: { data: feeData } as RequestResult<BatchFeeComponent[]>,
    } as unknown as ReturnType<typeof swr.default>);
    vi.spyOn(priceService, 'useEthPrice').mockReturnValue({
      data: 1,
    } as unknown as ReturnType<typeof priceService.useEthPrice>);

    const html = renderToStaticMarkup(
      React.createElement(IncomeChart, {
        timeRange: '1h',
      }),
    );

    expect(html).toContain('recharts-responsive-container');
  });
});
