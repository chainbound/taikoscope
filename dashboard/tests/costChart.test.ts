import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import * as swr from 'swr';
vi.mock('swr', () => ({ default: vi.fn() }));
import { CostChart } from '../components/CostChart';
import * as priceService from '../services/priceService';

const batchData = {
  batches: [
    {
      batch_id: 1,
      l1_block_number: 100,
      batch_size: 1024,
      last_l2_block_number: 200,
      first_l2_block_number: 190,
      proposer_addr: '0x123',
      total_priority_fee: 1e18,
      total_base_fee: 1e18,
      total_l1_data_cost: 0.5e18,
      net_profit: 1.5e18,
      total_transactions: 100,
      total_gas_used: 1000000,
      proposed_at: new Date().toISOString()
    },
  ]
};

describe('CostChart', () => {
  it('renders with cost data', () => {
    vi.mocked(swr.default).mockReturnValue({ data: { data: batchData } } as any);
    vi.spyOn(priceService, 'useEthPrice').mockReturnValue({ data: 1 } as any);

    const html = renderToStaticMarkup(
      React.createElement(CostChart, {
        timeRange: '1h',
        cloudCost: 100,
        proverCost: 100,
      }),
    );

    expect(html).toContain('recharts-responsive-container');
  });
});
