import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import * as swr from 'swr';
vi.mock('swr', () => ({ default: vi.fn() }));
import * as api from '../services/apiService';
import * as priceService from '../services/priceService';
import * as seqCfg from '../sequencerConfig';
import { ProfitRankingTable } from '../components/ProfitRankingTable';

describe('ProfitRankingTable', () => {
  it('renders sequencer profits', async () => {
    vi.mocked(swr.default)
      .mockReturnValueOnce({
        data: { data: [{ name: 'SeqA', value: 10, tps: null }] },
      } as any)
      .mockReturnValueOnce({
        data: {
          data: {
            priority_fee: 2e18,
            base_fee: 1e18,
            l1_data_cost: 0,
            sequencers: [
              {
                address: '0xseq',
                priority_fee: 2e18,
                base_fee: 1e18,
                l1_data_cost: 0,
              },
            ],
          },
        },
      } as any);
    vi.spyOn(api, 'fetchSequencerDistribution').mockResolvedValue({
      data: [{ name: 'SeqA', value: 10, tps: null }],
      badRequest: false,
      error: null,
    } as any);
    vi.spyOn(api, 'fetchL2Fees').mockResolvedValue({
      data: {
        priority_fee: 2e18,
        base_fee: 1e18,
        l1_data_cost: 0,
        sequencers: [
          {
            address: '0xseq',
            priority_fee: 2e18,
            base_fee: 1e18,
            l1_data_cost: 0,
          },
        ],
      },
      badRequest: false,
      error: null,
    } as any);
    vi.spyOn(priceService, 'useEthPrice').mockReturnValue({
      data: 1000,
    } as any);
    vi.spyOn(seqCfg, 'getSequencerAddress').mockReturnValue('0xseq');

    const html = renderToStaticMarkup(
      React.createElement(ProfitRankingTable, {
        timeRange: '1h',
        cloudCost: 0,
        proverCost: 0,
      }),
    );
    expect(html.includes('Sequencer Profit Ranking')).toBe(true);
    expect(html.includes('2,750')).toBe(true);
  });
});
