import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import * as swr from 'swr';
vi.mock('swr', () => ({ default: vi.fn() }));
import type {
  RequestResult,
  SequencerDistributionDataItem,
  L2FeesComponentsResponse,
} from '../services/apiService';
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
            { name: 'SeqB', address: '0xseqB', value: 5, tps: null },
          ],
        } as RequestResult<SequencerDistributionDataItem[]>,
      } as unknown as ReturnType<typeof swr.default>)
      .mockReturnValueOnce({
        data: {
          data: {
            priority_fee: 3e9,
            base_fee: 1.5e9,
            l1_data_cost: 0,
            prove_cost: 0,
            sequencers: [
              {
                address: '0xseqA',
                priority_fee: 2e9,
                base_fee: 1e9,
                l1_data_cost: 0,
                prove_cost: 0,
              },
              {
                address: '0xseqB',
                priority_fee: 1e9,
                base_fee: 0.5e9,
                l1_data_cost: 0,
                prove_cost: 0,
              },
            ],
            batches: [
              { sequencer: '0xseqA', batch_id: 1, revenue: 1e9, prove_cost: 0 } as any,
              { sequencer: '0xseqB', batch_id: 2, revenue: 0.5e9, prove_cost: 0 } as any,
            ],
          },
        } as unknown as RequestResult<L2FeesComponentsResponse>,
      } as unknown as ReturnType<typeof swr.default>);

    vi.spyOn(api, 'fetchSequencerDistribution').mockResolvedValue({
      data: [
        { name: 'SeqA', address: '0xseqA', value: 10, tps: null },
        { name: 'SeqB', address: '0xseqB', value: 5, tps: null },
      ],
      badRequest: false,
      error: null,
    } as RequestResult<SequencerDistributionDataItem[]>);
    vi.spyOn(api, 'fetchL2FeesComponents').mockResolvedValue({
      data: {
        priority_fee: 3e9,
        base_fee: 1.5e9,
        l1_data_cost: 0,
        prove_cost: 0,
        sequencers: [
          {
            address: '0xseqA',
            priority_fee: 2e9,
            base_fee: 1e9,
            l1_data_cost: 0,
            prove_cost: 0,
          },
          {
            address: '0xseqB',
            priority_fee: 1e9,
            base_fee: 0.5e9,
            l1_data_cost: 0,
            prove_cost: 0,
          },
        ],
        batches: [
          { sequencer: '0xseqA', batch_id: 1, revenue: 1e9, prove_cost: 0 } as any,
          { sequencer: '0xseqB', batch_id: 2, revenue: 0.5e9, prove_cost: 0 } as any,
        ],
      },
      badRequest: false,
      error: null,
    });
    vi.spyOn(priceService, 'useEthPrice').mockReturnValue({
      data: 1000,
    } as unknown as ReturnType<typeof priceService.useEthPrice>);

    const html = renderToStaticMarkup(
      React.createElement(ProfitRankingTable, {
        timeRange: '1h',
        cloudCost: 0,
        proverCost: 0,
      }),
    );
    expect(html.includes('Sequencer Profit Ranking')).toBe(true);
    expect(html.includes('2,750')).toBe(true);
    const firstSeqIdx = html.indexOf('0xseqA');
    const secondSeqIdx = html.indexOf('0xseqB');
    expect(firstSeqIdx).toBeGreaterThan(-1);
    expect(secondSeqIdx).toBeGreaterThan(firstSeqIdx);
    expect(html.includes('Revenue')).toBe(true);
    expect(html.includes('Batches')).toBe(true);
    expect(html.includes('Cost')).toBe(true);
    expect(html.includes('Profit')).toBe(true);
    expect(html.includes('Revenue-to-Cost Ratio')).toBe(true);
    expect(html.includes('↓')).toBe(true);
  });

  it('adds l1 cost to cost column', async () => {
    vi.mocked(swr.default)
      .mockReturnValueOnce({
        data: {
          data: [{ name: 'SeqA', address: '0xseqA', value: 1, tps: null }],
        } as RequestResult<SequencerDistributionDataItem[]>,
      } as unknown as ReturnType<typeof swr.default>)
      .mockReturnValueOnce({
        data: {
          data: {
            priority_fee: 1e9,
            base_fee: 0,
            l1_data_cost: 5e8,
            prove_cost: 1e7,
            sequencers: [
              {
                address: '0xseqA',
                priority_fee: 1e9,
                base_fee: 0,
                l1_data_cost: 5e8,
                prove_cost: 1e7,
              },
            ],
            batches: [
              { sequencer: '0xseqA', batch_id: 1, revenue: 1e9, prove_cost: 1e7 } as any,
            ],
          },
        } as unknown as RequestResult<L2FeesComponentsResponse>,
      } as unknown as ReturnType<typeof swr.default>);

    vi.spyOn(api, 'fetchSequencerDistribution').mockResolvedValue({
      data: [{ name: 'SeqA', address: '0xseqA', value: 1, tps: null }],
      badRequest: false,
      error: null,
    } as RequestResult<SequencerDistributionDataItem[]>);
    vi.spyOn(api, 'fetchL2FeesComponents').mockResolvedValue({
      data: {
        priority_fee: 1e9,
        base_fee: 0,
        l1_data_cost: 5e8,
        prove_cost: 1e7,
        sequencers: [
          {
            address: '0xseqA',
            priority_fee: 1e9,
            base_fee: 0,
            l1_data_cost: 5e8,
            prove_cost: 1e7,
          },
        ],
        batches: [
          { sequencer: '0xseqA', batch_id: 1, revenue: 1e9, prove_cost: 1e7 } as any,
        ],
      },
      badRequest: false,
      error: null,
    });
    vi.spyOn(priceService, 'useEthPrice').mockReturnValue({
      data: 100,
    } as unknown as ReturnType<typeof priceService.useEthPrice>);

    const html = renderToStaticMarkup(
      React.createElement(ProfitRankingTable, {
        timeRange: '1h',
        cloudCost: 0,
        proverCost: 0,
      }),
    );
    expect(html.includes('title="$')).toBe(true);
  });
});
