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
  fetchProveTimesAggregated,
  fetchVerifyTimes,
  fetchVerifyTimesAggregated,
  fetchAllBlockTransactions,
  fetchBlockTransactionsAggregated,
  fetchL2GasUsed,
  fetchL2GasUsedAggregated,
  fetchL1DataCost,
  fetchSequencerDistribution,
  fetchL2Tps,
  fetchL2TpsAggregated,
} from '../services/apiService';
import { getSequencerName } from '../sequencerConfig';
import {
  bytesToHex,
  blockLink,
  addressLink,
  formatDateTime,
  formatEth,
} from '../utils';
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
    columns: [{ key: 'block_number', label: 'L2 Block Number' }],
    mapData: (data) => data.map((b) => ({ block_number: blockLink(b) })),
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
      { key: 'old_sequencer', label: 'Old Sequencer' },
      { key: 'new_sequencer', label: 'New Sequencer' },
    ],
    mapData: (data) =>
      (data as L2ReorgEvent[]).map((e) => ({
        timestamp: formatDateTime(e.timestamp),
        l2_block_number: blockLink(e.l2_block_number),
        depth: e.depth.toLocaleString(),
        old_sequencer: getSequencerName(e.old_sequencer),
        new_sequencer: getSequencerName(e.new_sequencer),
      })),
    urlKey: 'reorgs',
    reverseOrder: false,
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
      { key: 'block_number', label: 'L1 Block' },
      { key: 'batch', label: 'Batch' },
      { key: 'blobs', label: 'Blobs' },
    ],
    mapData: (data) =>
      (data as Record<string, any>[]).map((d) => ({
        block_number: blockLink(d.block_number as number),
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
    reverseOrder: false,
    supportsPagination: true,
  },

  'block-tx': {
    title: 'Tx Count Per L2 Block',
    description: 'Transactions included in each L2 block.',
    fetcher: fetchAllBlockTransactions,
    aggregatedFetcher: fetchBlockTransactionsAggregated,
    columns: [
      { key: 'block_number', label: 'L2 Block Number' },
      { key: 'txs', label: 'Tx Count' },
    ],
    mapData: (data) =>
      (data as { block_number: number; txs: number }[]).map(
        (d) => ({
          block_number: blockLink(d.block_number),
          txs: d.txs.toLocaleString(),
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
    title: 'Proposing Cost',
    description: 'Data posting cost for each L1 block.',
    fetcher: fetchL1DataCost,
    columns: [
      { key: 'block_number', label: 'L1 Block' },
      { key: 'cost', label: 'Cost' },
    ],
    mapData: (data) =>
      (data as { block_number: number; cost: number }[]).map((d) => ({
        block_number: blockLink(d.block_number),
        cost: formatEth(d.cost, 4),
      })),
    urlKey: 'l1-data-cost',
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
    aggregatedFetcher: fetchL2TpsAggregated,
    columns: [
      { key: 'block_number', label: 'Block Number' },
      { key: 'tps', label: 'TPS' },
    ],
    mapData: (data) =>
      (data as { block_number: number; tps: number }[]).map((d) => ({
        block_number: blockLink(d.block_number),
        tps: d.tps.toFixed(2),
      })),
    chart: (data) => {
      const TpsChart = React.lazy(() =>
        import('../components/TpsChart').then((m) => ({
          default: m.TpsChart,
        })),
      );
      return React.createElement(TpsChart, {
        data,
        lineColor: '#4E79A7',
      });
    },
    urlKey: 'l2-tps',
    supportsPagination: true,
  },

  'prove-times': {
    title: 'Prove Times',
    description: 'Time taken to prove batches',
    fetcher: fetchProveTimes,
    aggregatedFetcher: fetchProveTimesAggregated,
    columns: [
      { key: 'name', label: 'Batch' },
      { key: 'value', label: 'Time' },
    ],
    mapData: (data) =>
      (data as Record<string, string | number>[]).map((d) => ({
        ...d,
        name: Number(d.name).toLocaleString(),
        value: `${Math.round(Number(d.value) / 60)} minutes`,
      })),
    urlKey: 'prove-times',
    reverseOrder: false,
    supportsPagination: true,
  },

  'verify-times': {
    title: 'Verify Times',
    description: 'Time taken to verify batches',
    fetcher: fetchVerifyTimes,
    aggregatedFetcher: fetchVerifyTimesAggregated,
    columns: [
      { key: 'name', label: 'Batch' },
      { key: 'value', label: 'Time' },
    ],
    mapData: (data) =>
      (data as Record<string, string | number>[]).map((d) => ({
        ...d,
        name: Number(d.name).toLocaleString(),
        value: `${Math.round(Number(d.value) / 60)} minutes`,
      })),
    urlKey: 'verify-times',
    reverseOrder: false,
    supportsPagination: true,
  },

  'batch-posting-times': {
    title: 'Batch Posting Times',
    description: 'Time taken to post batches',
    fetcher: fetchBatchPostingTimes,
    columns: [
      { key: 'value', label: 'Batch ID' },
      { key: 'timestamp', label: 'Minutes' },
    ],
    mapData: (data) =>
      (data as Record<string, any>[]).map((d) => ({
        value: Number(d.value).toLocaleString(),
        timestamp: `${Math.round(d.timestamp / 60)} minutes`,
      })),
    urlKey: 'batch-posting-times',
    reverseOrder: true,
    supportsPagination: true,
  },
};
