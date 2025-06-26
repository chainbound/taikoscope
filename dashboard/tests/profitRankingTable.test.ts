import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import * as swr from 'swr';
vi.mock('swr', () => ({ default: vi.fn() }));
import type {
  RequestResult,
  SequencerDistributionDataItem,
  L2FeesResponse,
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
      .mockReturnValueOnce({ data: new Map() } as unknown as ReturnType<typeof swr.default>)
      .mockReturnValueOnce({ data: new Map() } as unknown as ReturnType<typeof swr.default>)
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
        } as RequestResult<L2FeesResponse>,
      } as unknown as ReturnType<typeof swr.default>)
      .mockReturnValueOnce({
        data: new Map([
          ['0xseqa', 1],
          ['0xseqb', 1],
        ]),
      } as unknown as ReturnType<typeof swr.default>);

    vi.spyOn(api, 'fetchSequencerDistribution').mockResolvedValue({
      data: [
        { name: 'SeqA', address: '0xseqA', value: 10, tps: null },
        { name: 'SeqB', address: '0xseqB', value: 5, tps: null },
      ],
      badRequest: false,
      error: null,
    } as RequestResult<SequencerDistributionDataItem[]>);
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
    } as RequestResult<L2FeesResponse>);
    vi.spyOn(api, 'fetchProveCostsByProposer').mockResolvedValue({
      data: [],
      badRequest: false,
      error: null,
    } as RequestResult<api.SequencerCostItem[]>);
    vi.spyOn(api, 'fetchVerifyCostsByProposer').mockResolvedValue({
      data: [],
      badRequest: false,
      error: null,
    } as RequestResult<api.SequencerCostItem[]>);
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
    expect(html.includes('Income/Cost')).toBe(true);
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
        data: new Map([['0xseqa', 1e16]]),
      } as unknown as ReturnType<typeof swr.default>)
      .mockReturnValueOnce({
        data: new Map([['0xseqa', 2e16]]),
      } as unknown as ReturnType<typeof swr.default>)
      .mockReturnValueOnce({
        data: {
          data: {
            priority_fee: 1e18,
            base_fee: 0,
            l1_data_cost: 5e17,
            sequencers: [
              {
                address: '0xseqA',
                priority_fee: 1e18,
                base_fee: 0,
                l1_data_cost: 5e17,
              },
            ],
          },
        } as RequestResult<L2FeesResponse>,
      } as unknown as ReturnType<typeof swr.default>)
      .mockReturnValueOnce({
        data: new Map([['0xseqa', 1]])
      } as unknown as ReturnType<typeof swr.default>);

    vi.spyOn(api, 'fetchSequencerDistribution').mockResolvedValue({
      data: [{ name: 'SeqA', address: '0xseqA', value: 1, tps: null }],
      badRequest: false,
      error: null,
    } as RequestResult<SequencerDistributionDataItem[]>);
    vi.spyOn(api, 'fetchL2Fees').mockResolvedValue({
      data: {
        priority_fee: 1e18,
        base_fee: 0,
        l1_data_cost: 5e17,
        sequencers: [
          {
            address: '0xseqA',
            priority_fee: 1e18,
            base_fee: 0,
            l1_data_cost: 5e17,
          },
        ],
      },
      badRequest: false,
      error: null,
    } as RequestResult<L2FeesResponse>);
    vi.spyOn(api, 'fetchProveCostsByProposer').mockResolvedValue({
      data: [{ address: '0xseqA', cost: 1e16 }],
      badRequest: false,
      error: null,
    } as RequestResult<api.SequencerCostItem[]>);
    vi.spyOn(api, 'fetchVerifyCostsByProposer').mockResolvedValue({
      data: [{ address: '0xseqA', cost: 2e16 }],
      badRequest: false,
      error: null,
    } as RequestResult<api.SequencerCostItem[]>);
    vi.spyOn(priceService, 'useEthPrice').mockReturnValue({
      data: 100,
    } as unknown as ReturnType<typeof priceService.useEthPrice>);

    const html = renderToStaticMarkup(
      React.createElement(ProfitRankingTable, {
        timeRange: '1h',
        cloudCost: 0,
        proverCost: 0,
        proveCost: 1,
        verifyCost: 2,
      }),
    );
    expect(html.includes('title="$53.00"')).toBe(true);
  });
});
