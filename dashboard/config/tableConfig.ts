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
  fetchActiveGatewayAddresses,
  fetchBatchBlobCounts,
  fetchBatchPostingTimes,
  fetchProveTimes,
  fetchVerifyTimes,
  fetchBlockTransactions,
  fetchL2BlockTimes,
  fetchL1BlockTimes,
  fetchSequencerDistribution,
} from '../services/apiService';
import { getSequencerName } from '../sequencerConfig';
import { bytesToHex } from '../utils';
import { TAIKO_PINK } from '../theme';
import React from 'react';

export interface TableColumn {
  key: string;
  label: string;
}

export interface TableConfig {
  title: string | ((params: Record<string, any>) => string);
  fetcher: (range: TimeRange, ...args: any[]) => Promise<any>;
  columns: TableColumn[];
  mapData?: (
    data: any[],
    params?: Record<string, any>,
  ) => Record<string, string | number>[];
  chart?: (data: any[]) => React.ReactNode;
  supportsPagination?: boolean;
  urlKey: string;
}

export const TABLE_CONFIGS: Record<string, TableConfig> = {
  'sequencer-blocks': {
    title: (params) => `Blocks proposed by ${getSequencerName(params.address)}`,
    fetcher: fetchSequencerBlocks,
    columns: [{ key: 'block', label: 'Block Number' }],
    mapData: (data) => data.map((b) => ({ block: b })),
    urlKey: 'sequencer-blocks',
  },

  reorgs: {
    title: 'L2 Reorgs',
    fetcher: fetchL2ReorgEvents,
    columns: [
      { key: 'timestamp', label: 'Time' },
      { key: 'l2_block_number', label: 'Block Number' },
      { key: 'depth', label: 'Depth' },
    ],
    mapData: (data) =>
      (data as L2ReorgEvent[]).map((e) => ({
        timestamp: new Date(e.timestamp).toLocaleString(),
        l2_block_number: e.l2_block_number,
        depth: e.depth,
      })),
    chart: (data) => {
      const ReorgDepthChart = React.lazy(() =>
        import('../components/ReorgDepthChart').then((m) => ({
          default: m.ReorgDepthChart,
        })),
      );
      return React.createElement(ReorgDepthChart, {
        data: data as L2ReorgEvent[],
      });
    },
    urlKey: 'reorgs',
  },

  slashings: {
    title: 'Slashing Events',
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
  },

  'forced-inclusions': {
    title: 'Forced Inclusions',
    fetcher: fetchForcedInclusionEvents,
    columns: [{ key: 'blob_hash', label: 'Blob Hash' }],
    mapData: (data) =>
      (data as ForcedInclusionEvent[]).map((e) => ({
        blob_hash: bytesToHex(e.blob_hash),
      })),
    urlKey: 'forced-inclusions',
  },

  gateways: {
    title: 'Active Sequencers',
    fetcher: fetchActiveGatewayAddresses,
    columns: [{ key: 'address', label: 'Address' }],
    mapData: (data) => data.map((g) => ({ address: g })),
    urlKey: 'gateways',
  },

  'blobs-per-batch': {
    title: 'Blobs per Batch',
    fetcher: fetchBatchBlobCounts,
    columns: [
      { key: 'batch', label: 'Batch' },
      { key: 'blobs', label: 'Blobs' },
    ],
    mapData: (data) => data as Record<string, string | number>[],
    urlKey: 'blobs-per-batch',
  },

  'batch-posting-cadence': {
    title: 'Batch Posting Cadence',
    fetcher: fetchBatchPostingTimes,
    columns: [
      { key: 'value', label: 'Batch' },
      { key: 'timestamp', label: 'Interval (ms)' },
    ],
    mapData: (data) => data as Record<string, string | number>[],
    chart: (data) => {
      const BlockTimeChart = React.lazy(() =>
        import('../components/BlockTimeChart').then((m) => ({
          default: m.BlockTimeChart,
        })),
      );
      return React.createElement(BlockTimeChart, {
        data,
        lineColor: '#FF9DA7',
      });
    },
    urlKey: 'batch-posting-cadence',
  },

  'prove-time': {
    title: 'Prove Time',
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
  },

  'verify-time': {
    title: 'Verify Time',
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
  },

  'block-tx': {
    title: 'Tx Count Per Block',
    fetcher: (range) => fetchBlockTransactions(range, 50),
    columns: [
      { key: 'block', label: 'Block Number' },
      { key: 'txs', label: 'Tx Count' },
      { key: 'sequencer', label: 'Sequencer' },
    ],
    mapData: (data) => data as Record<string, string | number>[],
    chart: (data) => {
      const BlockTxChart = React.lazy(() =>
        import('../components/BlockTxChart').then((m) => ({
          default: m.BlockTxChart,
        })),
      );
      return React.createElement(BlockTxChart, { data, barColor: '#4E79A7' });
    },
    urlKey: 'block-tx',
  },

  'l2-block-times': {
    title: 'L2 Block Times',
    fetcher: fetchL2BlockTimes,
    columns: [
      { key: 'value', label: 'Block Number' },
      { key: 'timestamp', label: 'Interval (ms)' },
    ],
    mapData: (data) => data as Record<string, string | number>[],
    chart: (data) => {
      const BlockTimeChart = React.lazy(() =>
        import('../components/BlockTimeChart').then((m) => ({
          default: m.BlockTimeChart,
        })),
      );
      return React.createElement(BlockTimeChart, {
        data,
        lineColor: '#FAA43A',
      });
    },
    urlKey: 'l2-block-times',
  },

  'l1-block-times': {
    title: 'L1 Block Times',
    fetcher: fetchL1BlockTimes,
    columns: [
      { key: 'value', label: 'Block Number' },
      { key: 'timestamp', label: 'Interval (ms)' },
    ],
    mapData: (data) => data as Record<string, string | number>[],
    chart: (data) => {
      const BlockTimeChart = React.lazy(() =>
        import('../components/BlockTimeChart').then((m) => ({
          default: m.BlockTimeChart,
        })),
      );
      return React.createElement(BlockTimeChart, {
        data,
        lineColor: '#60BD68',
      });
    },
    urlKey: 'l1-block-times',
  },

  'sequencer-dist': {
    title: 'Sequencer Distribution',
    fetcher: fetchSequencerDistribution,
    columns: [
      { key: 'name', label: 'Sequencer' },
      { key: 'value', label: 'Blocks' },
    ],
    mapData: (data) => data as Record<string, string | number>[],
    supportsPagination: true,
    urlKey: 'sequencer-dist',
  },
};
