import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import * as swr from 'swr';
vi.mock('swr', () => ({ default: vi.fn() }));
import type { RequestResult } from '../services/apiService';
import type { BatchFeeComponent } from '../types';
import * as priceService from '../services/priceService';
import { BlockProfitTables } from '../components/BlockProfitTables';

const feeData = [
  {
    batch: 1,
    l1Block: 1,
    sequencer: 'SeqA',
    priority: '1000000000000000000',
    base: '1000000000000000000',
    l1Cost: '0',
    amortizedProveCost: '0',

  },
];

describe('BlockProfitTables', () => {
  it('renders with prove cost', () => {
    vi.mocked(swr.default).mockReturnValue({
      data: { data: feeData } as RequestResult<BatchFeeComponent[]>,
    } as unknown as ReturnType<typeof swr.default>);
    vi.spyOn(priceService, 'useEthPrice').mockReturnValue({
      data: 1,
    } as unknown as ReturnType<typeof priceService.useEthPrice>);

    const html = renderToStaticMarkup(
      React.createElement(BlockProfitTables, {
        timeRange: '1h',
        cloudCost: 100,
        proverCost: 100,
      }),
    );

    expect(html).toContain('Top 5 Profitable Batches');
  });
});
