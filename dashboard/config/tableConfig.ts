import {
  TimeRange,
  L2ReorgEvent,
  SlashingEvent,
  ForcedInclusionEvent,
} from '../types';
import {
  fetchSequencerBlocks,
  fetchL2ReorgEvents,
  fetchSlashingEvents,
  fetchForcedInclusionEvents,
  fetchActiveSequencerAddresses,
  fetchBatchBlobCounts,
  fetchBatchPostingTimes,
  fetchProveTimes,
  fetchVerifyTimes,
  fetchAllBlockTransactions,
  fetchBlockTransactionsAggregated,
  fetchL2BlockTimes,
  fetchL2BlockTimesAggregated,
  fetchL2GasUsed,
  fetchL2GasUsedAggregated,
  fetchL1DataCost,
  fetchProveCost,
  fetchVerifyCost,
  fetchSequencerDistribution,
  fetchL2Tps,
} from '../services/apiService';
import { getSequencerName } from '../sequencerConfig';
import {
  bytesToHex,
  blockLink,
  addressLink,
  formatDateTime,
  formatEth,
} from '../utils';
import { TAIKO_PINK } from '../theme';
import React from 'react';

export interface TableColumn {
  key: string;
  label: string;
  sortable?: boolean;
}

export interface TableConfig {
  title: string | ((params: Record<string, any>) => string);
  description?: string | React.ReactNode;
  fetcher: (range: TimeRange, ...args: any[]) => Promise<any>;
  aggregatedFetcher?: (range: TimeRange, ...args: any[]) => Promise<any>;
  columns: TableColumn[];
  mapData?: (
    data: any[],
    params?: Record<string, any>,
  ) => Record<string, React.ReactNode | string | number>[];
  chart?: (data: any[]) => React.ReactNode;
  supportsPagination?: boolean;
  urlKey: string;
  useUnlimitedData?: boolean;
  reverseOrder?: boolean;
}

export const TABLE_CONFIGS: Record<string, TableConfig> = {
  'sequencer-blocks': {
    title: (params) => `Blocks proposed by ${getSequencerName(params.address)}`,
    description: 'Blocks proposed by the given sequencer.',
    fetcher: fetchSequencerBlocks,
    columns: [{ key: 'block', label: 'L2 Block Number' }],
    mapData: (data) => data.map((b) => ({ block: blockLink(b) })),
    urlKey: 'sequencer-blocks',
  },

  reorgs: {
    title: 'L2 Reorgs',
    description:
      'An L2 reorg occurs when the chain replaces previously published blocks. Depth shows how many blocks were replaced.',
    fetcher: fetchL2ReorgEvents,
    columns: [
      { key: 'timestamp', label: 'Time' },
      { key: 'l2_block_number', label: 'L2 Block Number' },
      { key: 'depth', label: 'Depth' },
    ],
    mapData: (data) =>
      (data as L2ReorgEvent[]).map((e) => ({
        timestamp: formatDateTime(e.timestamp),
        l2_block_number: blockLink(e.l2_block_number),
        depth: e.depth.toLocaleString(),
      })),
    urlKey: 'reorgs',
    reverseOrder: true,
    supportsPagination: true,
  },

  slashings: {
    title: 'Slashing Events',
    description: 'Validators that have been slashed on L1.',
    fetcher: fetchSlashingEvents,
    columns: [
      { key: 'l1_block_number', label: 'L1 Block' },
      { key: 'validator_addr', label: 'Validator' },
    ],
    mapData: (data) =>
      (data as SlashingEvent[]).map((e) => ({
        l1_block_number: e.l1_block_number,
        validator_addr: bytesToHex(e.validator_addr),
      })),
    urlKey: 'slashings',
    reverseOrder: true,
  },

  'forced-inclusions': {
    title: 'Forced Inclusions',
    description: 'Batches that were forcibly included.',
    fetcher: fetchForcedInclusionEvents,
    columns: [{ key: 'blob_hash', label: 'Blob Hash' }],
    mapData: (data) =>
      (data as ForcedInclusionEvent[]).map((e) => ({
        blob_hash: bytesToHex(e.blob_hash),
      })),
    urlKey: 'forced-inclusions',
    reverseOrder: true,
  },

  gateways: {
    title: 'Active Sequencers',
    description: 'Current candidates to be the sequencer.',
    fetcher: fetchActiveSequencerAddresses,
    columns: [
      { key: 'sequencer', label: 'Sequencer' },
      { key: 'address', label: 'Address' },
    ],
    mapData: (data) =>
      data.map((g) => {
        const name = getSequencerName(g);
        return {
          sequencer: name === g ? 'Unknown' : name,
          address: addressLink(g),
        };
      }),
    urlKey: 'gateways',
  },

  'blobs-per-batch': {
    title: 'Blobs per Batch',
    description: 'Number of blobs posted with each batch.',
    fetcher: fetchBatchBlobCounts,
    columns: [
      { key: 'block', label: 'L1 Block' },
      { key: 'batch', label: 'Batch' },
      { key: 'blobs', label: 'Blobs' },
    ],
    mapData: (data) =>
      (data as Record<string, any>[]).map((d) => ({
        block: blockLink(d.block as number),
        batch: d.batch.toLocaleString(),
        blobs: d.blobs.toLocaleString(),
      })),
    urlKey: 'blobs-per-batch',
    supportsPagination: true,
  },

  'batch-posting-cadence': {
    title: 'Batch Posting Cadence',
    description: 'Time between batches posted on L1.',
    fetcher: fetchBatchPostingTimes,
    columns: [
      { key: 'value', label: 'Batch ID' },
      { key: 'timestamp', label: 'Interval (s)' },
    ],
    mapData: (data) =>
      (data as Record<string, any>[]).map((d) => ({
        value: Number(d.value).toLocaleString(),
        timestamp: d.timestamp,
      })),
    urlKey: 'batch-posting-cadence',
    reverseOrder: true,
    supportsPagination: true,
  },

  'prove-time': {
    title: 'Prove Time',
    description: 'How long it took to prove each batch.',
    fetcher: fetchProveTimes,
    columns: [
      { key: 'name', label: 'Batch' },
      { key: 'value', label: 'Seconds' },
    ],
    mapData: (data) =>
      (data as Record<string, string | number>[]).map((d) => ({
        ...d,
        name: Number(d.name).toLocaleString(),
        value: typeof d.value === 'number' ? d.value.toLocaleString() : d.value,
      })),
    chart: (data) => {
      const BatchProcessChart = React.lazy(() =>
        import('../components/BatchProcessChart').then((m) => ({
          default: m.BatchProcessChart,
        })),
      );
      return React.createElement(BatchProcessChart, {
        data,
        lineColor: TAIKO_PINK,
      });
    },
    urlKey: 'prove-time',
    reverseOrder: true,
    supportsPagination: true,
  },

  'verify-time': {
    title: 'Verify Time',
    description: 'How long it took to verify each batch.',
    fetcher: fetchVerifyTimes,
    columns: [
      { key: 'name', label: 'Batch' },
      { key: 'value', label: 'Seconds' },
    ],
    mapData: (data) =>
      (data as Record<string, string | number>[]).map((d) => ({
        ...d,
        name: Number(d.name).toLocaleString(),
        value: typeof d.value === 'number' ? d.value.toLocaleString() : d.value,
      })),
    chart: (data) => {
      const BatchProcessChart = React.lazy(() =>
        import('../components/BatchProcessChart').then((m) => ({
          default: m.BatchProcessChart,
        })),
      );
      return React.createElement(BatchProcessChart, {
        data,
        lineColor: '#5DA5DA',
      });
    },
    urlKey: 'verify-time',
    reverseOrder: true,
    supportsPagination: true,
  },

  'block-tx': {
    title: 'Tx Count Per L2 Block',
    description: 'Transactions included in each L2 block.',
    fetcher: fetchAllBlockTransactions,
    aggregatedFetcher: fetchBlockTransactionsAggregated,
    columns: [
      { key: 'block', label: 'L2 Block Number' },
      { key: 'txs', label: 'Tx Count' },
      { key: 'sequencer', label: 'Sequencer' },
    ],
    mapData: (data) =>
      (data as { block: number; txs: number; sequencer: string }[]).map(
        (d) => ({
          block: blockLink(d.block),
          txs: d.txs.toLocaleString(),
          sequencer: addressLink(d.sequencer),
        }),
      ),
    chart: (data) => {
      const BlockTxChart = React.lazy(() =>
        import('../components/BlockTxChart').then((m) => ({
          default: m.BlockTxChart,
        })),
      );
      return React.createElement(BlockTxChart, {
        data,
        lineColor: '#4E79A7',
      });
    },
    urlKey: 'block-tx',
    supportsPagination: true,
  },

  'l2-block-times': {
    title: 'L2 Block Times',
    description: 'Interval between consecutive L2 blocks.',
    fetcher: fetchL2BlockTimes,
    aggregatedFetcher: fetchL2BlockTimesAggregated,
    columns: [
      { key: 'value', label: 'L2 Block Number' },
      { key: 'timestamp', label: 'Interval (s)' },
    ],
    mapData: (data) =>
      (data as { value: number; timestamp: number }[]).map((d) => ({
        value: blockLink(d.value),
        timestamp: d.timestamp.toLocaleString(),
      })),
    chart: (data) => {
      const BlockTimeDistributionChart = React.lazy(() =>
        import('../components/BlockTimeDistributionChart').then((m) => ({
          default: m.BlockTimeDistributionChart,
        })),
      );
      return React.createElement(BlockTimeDistributionChart, {
        data,
        barColor: '#FAA43A',
      });
    },
    urlKey: 'l2-block-times',
    reverseOrder: true,
    supportsPagination: true,
  },

  'l2-gas-used': {
    title: 'Gas Used Per Block',
    description: 'Gas used by each block.',
    fetcher: fetchL2GasUsed,
    aggregatedFetcher: fetchL2GasUsedAggregated,
    columns: [
      { key: 'value', label: 'Block Number' },
      { key: 'timestamp', label: 'Gas Used' },
    ],
    mapData: (data) =>
      (data as { value: number; timestamp: number }[]).map((d) => ({
        value: blockLink(d.value),
        timestamp: d.timestamp.toLocaleString(),
      })),
    chart: (data) => {
      const GasUsedChart = React.lazy(() =>
        import('../components/GasUsedChart').then((m) => ({
          default: m.GasUsedChart,
        })),
      );
      return React.createElement(GasUsedChart, {
        data,
        lineColor: '#E573B5',
      });
    },
    urlKey: 'l2-gas-used',
    reverseOrder: false,
    supportsPagination: true,
  },

  'l1-data-cost': {
    title: 'L1 Data Cost',
    description: 'Data posting cost for each L1 block.',
    fetcher: fetchL1DataCost,
    columns: [
      { key: 'block', label: 'L1 Block' },
      { key: 'cost', label: 'Cost' },
    ],
    mapData: (data) =>
      (data as { block: number; cost: number }[]).map((d) => ({
        block: blockLink(d.block),
        cost: formatEth(d.cost),
      })),
    urlKey: 'l1-data-cost',
    supportsPagination: true,
  },

  'prove-cost': {
    title: 'Prove Cost',
    description: 'Cost to prove each batch.',
    fetcher: fetchProveCost,
    columns: [
      { key: 'batch', label: 'Batch' },
      { key: 'cost', label: 'Cost' },
    ],
    mapData: (data) =>
      (data as { batch: number; cost: number }[]).map((d) => ({
        batch: d.batch.toLocaleString(),
        cost: formatEth(d.cost),
      })),
    urlKey: 'prove-cost',
    supportsPagination: true,
  },

  'verify-cost': {
    title: 'Verify Cost',
    description: 'Cost to verify each batch.',
    fetcher: fetchVerifyCost,
    columns: [
      { key: 'batch', label: 'Batch' },
      { key: 'cost', label: 'Cost' },
    ],
    mapData: (data) =>
      (data as { batch: number; cost: number }[]).map((d) => ({
        batch: d.batch.toLocaleString(),
        cost: formatEth(d.cost),
      })),
    urlKey: 'verify-cost',
    supportsPagination: true,
  },

  'sequencer-dist': {
    title: 'Sequencer Distribution',
    description: 'Breakdown of blocks proposed by each sequencer.',
    fetcher: fetchSequencerDistribution,
    columns: [
      { key: 'name', label: 'Sequencer' },
      { key: 'value', label: 'Blocks', sortable: true },
      { key: 'tps', label: 'TPS', sortable: true },
    ],
    mapData: (data) =>
      (data as any[]).map((d) => ({
        ...d,
        name: addressLink(d.address as string, d.name as string),
        tps: d.tps != null ? d.tps.toFixed(2) : 'N/A',
      })),
    supportsPagination: true,
    urlKey: 'sequencer-dist',
  },

  'l2-tps': {
    title: 'Transactions Per Second',
    description: 'Transactions per second for each L2 block.',
    fetcher: fetchL2Tps,
    columns: [
      { key: 'block', label: 'Block Number' },
      { key: 'tps', label: 'TPS' },
    ],
    mapData: (data) =>
      (data as { block: number; tps: number }[]).map((d) => ({
        block: blockLink(d.block),
        tps: d.tps.toFixed(2),
      })),
    urlKey: 'l2-tps',
    supportsPagination: true,
  },
};
