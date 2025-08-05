import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import * as swr from 'swr';
vi.mock('swr', () => ({ default: vi.fn() }));
import type { RequestResult } from '../services/apiService';
import * as priceService from '../services/priceService';
import { EconomicsChart } from '../components/EconomicsChart';

const batchData = [{
  batch_id: 1,
  l1_tx_hash: '0x123',
  sequencer: '0xseq1',
  priority_fee: 100000000,
  base_fee: 200000000,
  l1_data_cost: 50000000,
  prove_cost: 25000000,
}];

describe('EconomicsChart', () => {
  it('renders with economics data', () => {
    vi.mocked(swr.default).mockReturnValue({
      data: { data: { batches: batchData } } as unknown as RequestResult<any>,
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
