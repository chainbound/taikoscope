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
  fetchL2BlockTimes,
  fetchL2GasUsed,
  fetchSequencerDistribution,
  fetchL2Tps,
} from '../services/apiService';
import { getSequencerName, getSequencerAddress } from '../sequencerConfig';
import { bytesToHex, blockLink, addressLink } from '../utils';
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
        timestamp: new Date(e.timestamp).toLocaleString(),
        l2_block_number: blockLink(e.l2_block_number),
        depth: e.depth,
      })),
    urlKey: 'reorgs',
    reverseOrder: true,
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
      data.map((g) => ({
        sequencer: getSequencerName(g),
        address: g,
      })),
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
        batch: d.batch,
        blobs: d.blobs,
      })),
    urlKey: 'blobs-per-batch',
  },

  'batch-posting-cadence': {
    title: 'Batch Posting Cadence',
    description: 'Time between batches posted on L1.',
    fetcher: fetchBatchPostingTimes,
    columns: [
      { key: 'value', label: 'Batch' },
      { key: 'timestamp', label: 'Interval (s)' },
    ],
    mapData: (data) =>
      (data as Record<string, any>[]).map((d) => ({
        value: blockLink(d.value as number),
        timestamp: d.timestamp,
      })),
    urlKey: 'batch-posting-cadence',
    reverseOrder: true,
  },

  'prove-time': {
    title: 'Prove Time',
    description: 'How long it took to prove each batch.',
    fetcher: fetchProveTimes,
    columns: [
      { key: 'name', label: 'Batch' },
      { key: 'value', label: 'Seconds' },
    ],
    mapData: (data) => data as Record<string, string | number>[],
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
  },

  'verify-time': {
    title: 'Verify Time',
    description: 'How long it took to verify each batch.',
    fetcher: fetchVerifyTimes,
    columns: [
      { key: 'name', label: 'Batch' },
      { key: 'value', label: 'Seconds' },
    ],
    mapData: (data) => data as Record<string, string | number>[],
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
  },

  'block-tx': {
    title: 'Tx Count Per L2 Block',
    description: 'Transactions included in each L2 block.',
    fetcher: fetchAllBlockTransactions,
    columns: [
      { key: 'block', label: 'L2 Block Number' },
      { key: 'txs', label: 'Tx Count' },
      { key: 'sequencer', label: 'Sequencer' },
    ],
    mapData: (data) => data as Record<string, string | number>[],
    urlKey: 'block-tx',
  },

  'l2-block-times': {
    title: 'L2 Block Times',
    description: 'Interval between consecutive L2 blocks.',
    fetcher: fetchL2BlockTimes,
    columns: [
      { key: 'value', label: 'L2 Block Number' },
      { key: 'timestamp', label: 'Interval (s)' },
    ],
    mapData: (data) => data as Record<string, string | number>[],
    urlKey: 'l2-block-times',
    reverseOrder: true,
  },

  'l2-gas-used': {
    title: 'Gas Used Per Block',
    description: 'Gas used by each block.',
    fetcher: fetchL2GasUsed,
    columns: [
      { key: 'value', label: 'Block Number' },
      { key: 'timestamp', label: 'Gas Used' },
    ],
    mapData: (data) =>
      (data as { value: number; timestamp: number }[]).map((d) => ({
        value: d.value.toLocaleString(),
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
    reverseOrder: true,
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
        name: addressLink(
          getSequencerAddress(d.name as string) ?? (d.name as string),
          d.name as string,
        ),
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
    reverseOrder: true,
  },
};
