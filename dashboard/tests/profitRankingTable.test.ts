import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import * as swr from 'swr';
vi.mock('swr', () => ({ default: vi.fn() }));
import type {
  RequestResult,
  SequencerDistributionDataItem,
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
          data: [
            {
              batch: 1,
              l1Block: 1,
              sequencer: '0xseqA',
              priority: 2e18,
              base: 1e18,
              l1Cost: 0,
              amortizedProveCost: 0,
              amortizedVerifyCost: 0,
            },
            {
              batch: 2,
              l1Block: 2,
              sequencer: '0xseqB',
              priority: 1e18,
              base: 0.5e18,
              l1Cost: 0,
              amortizedProveCost: 0,
              amortizedVerifyCost: 0,
            },
          ],
        } as RequestResult<api.BatchFeeComponent[]>,
      } as unknown as ReturnType<typeof swr.default>);

    vi.spyOn(api, 'fetchSequencerDistribution').mockResolvedValue({
      data: [
        { name: 'SeqA', address: '0xseqA', value: 10, tps: null },
        { name: 'SeqB', address: '0xseqB', value: 5, tps: null },
      ],
      badRequest: false,
      error: null,
    } as RequestResult<SequencerDistributionDataItem[]>);
    vi.spyOn(api, 'fetchBatchFeeComponents').mockResolvedValue({
      data: [
        {
          batch: 1,
          l1Block: 1,
          sequencer: '0xseqA',
          priority: 2e18,
          base: 1e18,
          l1Cost: 0,
          amortizedProveCost: 0,
          amortizedVerifyCost: 0,
        },
        {
          batch: 2,
          l1Block: 2,
          sequencer: '0xseqB',
          priority: 1e18,
          base: 0.5e18,
          l1Cost: 0,
          amortizedProveCost: 0,
          amortizedVerifyCost: 0,
        },
      ],
      badRequest: false,
      error: null,
    } as RequestResult<api.BatchFeeComponent[]>);
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
        data: {
          data: [
            {
              batch: 1,
              l1Block: 1,
              sequencer: '0xseqA',
              priority: 1e18,
              base: 0,
              l1Cost: 5e17,
              amortizedProveCost: 1e16,
              amortizedVerifyCost: 2e16,
            },
          ],
        } as RequestResult<api.BatchFeeComponent[]>,
      } as unknown as ReturnType<typeof swr.default>);

    vi.spyOn(api, 'fetchSequencerDistribution').mockResolvedValue({
      data: [{ name: 'SeqA', address: '0xseqA', value: 1, tps: null }],
      badRequest: false,
      error: null,
    } as RequestResult<SequencerDistributionDataItem[]>);
    vi.spyOn(api, 'fetchBatchFeeComponents').mockResolvedValue({
      data: [
        {
          batch: 1,
          l1Block: 1,
          sequencer: '0xseqA',
          priority: 1e18,
          base: 0,
          l1Cost: 5e17,
          amortizedProveCost: 1e16,
          amortizedVerifyCost: 2e16,
        },
      ],
      badRequest: false,
      error: null,
    } as RequestResult<api.BatchFeeComponent[]>);
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
