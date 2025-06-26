import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import * as swr from 'swr';
vi.mock('swr', () => ({ default: vi.fn() }));
import * as priceService from '../services/priceService';
import { BlockProfitTables } from '../components/BlockProfitTables';

const feeData = [
  { batch: 1, l1Block: 1, sequencer: 'SeqA', priority: 1e18, base: 1e18, l1Cost: 0 },
];

describe('BlockProfitTables', () => {
  it('renders with prove and verify cost', () => {
    vi.mocked(swr.default).mockReturnValue({ data: { data: feeData } } as any);
    vi.spyOn(priceService, 'useEthPrice').mockReturnValue({ data: 1 } as any);

    const html = renderToStaticMarkup(
      React.createElement(BlockProfitTables, {
        timeRange: '1h',
        cloudCost: 100,
        proverCost: 100,
        proveCost: 5,
      }),
    );

    expect(html).toContain('Top 5 Profitable Batches');
  });
});
