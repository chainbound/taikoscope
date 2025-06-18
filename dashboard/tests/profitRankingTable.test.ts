import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import * as swr from 'swr';
vi.mock('swr', () => ({ default: vi.fn() }));
import * as api from '../services/apiService';
import * as priceService from '../services/priceService';
import { ProfitRankingTable } from '../components/ProfitRankingTable';

describe('ProfitRankingTable', () => {
  it('renders sequencer profits', async () => {
    vi.mocked(swr.default)
      .mockReturnValueOnce({
        data: {
          data: [
            { name: 'SeqA', address: '0xseqA', value: 10, tps: null },
            { name: 'SeqB', address: '0xseqB', value: 5, tps: null }
          ]
        },
      } as any)
      .mockReturnValueOnce({
        data: {
          data: {
            priority_fee: 3e18,
            base_fee: 1.5e18,
            l1_data_cost: 0,
            sequencers: [
              {
                address: '0xseqA',
                priority_fee: 2e18,
                base_fee: 1e18,
                l1_data_cost: 0,
              },
              {
                address: '0xseqB',
                priority_fee: 1e18,
                base_fee: 0.5e18,
                l1_data_cost: 0,
              },
            ],
          },
        },
      } as any);
    vi.spyOn(api, 'fetchSequencerDistribution').mockResolvedValue({
      data: [
        { name: 'SeqA', address: '0xseqA', value: 10, tps: null },
        { name: 'SeqB', address: '0xseqB', value: 5, tps: null }
      ],
      badRequest: false,
      error: null,
    } as any);
    vi.spyOn(api, 'fetchL2Fees').mockResolvedValue({
      data: {
        priority_fee: 3e18,
        base_fee: 1.5e18,
        l1_data_cost: 0,
        sequencers: [
          {
            address: '0xseqA',
            priority_fee: 2e18,
            base_fee: 1e18,
            l1_data_cost: 0,
          },
          {
            address: '0xseqB',
            priority_fee: 1e18,
            base_fee: 0.5e18,
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

    const html = renderToStaticMarkup(
      React.createElement(ProfitRankingTable, {
        timeRange: '1h',
        cloudCost: 0,
        proverCost: 0,
      }),
    );
    expect(html.includes('Sequencer Profit Ranking')).toBe(true);
    expect(html.includes('2,750')).toBe(true);
    const firstSeqIdx = html.indexOf('SeqA');
    const secondSeqIdx = html.indexOf('SeqB');
    expect(firstSeqIdx).toBeGreaterThan(-1);
    expect(secondSeqIdx).toBeGreaterThan(firstSeqIdx);
    expect(html.includes('Profit (USD)')).toBe(true);
    expect(html.includes('↓')).toBe(true);
  });
});
