import React from 'react';
import { type MetricData } from './types';
import { formatSeconds } from './utils.js';
import type { RequestResult } from './services/apiService';

export interface MetricInputData {
  l2Cadence: number | null;
  batchCadence: number | null;
  avgProve: number | null;
  avgVerify: number | null;
  activeGateways: number | null;
  currentOperator: string | null;
  nextOperator: string | null;
  l2Reorgs: number | null;
  slashings: number | null;
  forcedInclusions: number | null;
  l2Block: number | null;
  l1Block: number | null;
}

export const createMetrics = (data: MetricInputData): MetricData[] => [
  {
    title: 'L2 Block Cadence',
    value:
      data.l2Cadence != null ? formatSeconds(data.l2Cadence / 1000) : 'N/A',
  },
  {
    title: 'Batch Posting Cadence',
    value:
      data.batchCadence != null
        ? formatSeconds(data.batchCadence / 1000)
        : 'N/A',
  },
  {
    title: 'Avg. Prove Time',
    value:
      data.avgProve != null && data.avgProve > 0
        ? formatSeconds(data.avgProve / 1000)
        : 'N/A',
  },
  {
    title: React.createElement(
      'a',
      {
        href: 'https://docs.taiko.xyz/taiko-alethia-protocol/protocol-architecture/block-states',
        target: '_blank',
        rel: 'noopener noreferrer',
        className: 'hover:underline',
      },
      'Avg. Verify Time',
    ),
    value:
      data.avgVerify != null && data.avgVerify > 0
        ? formatSeconds(data.avgVerify / 1000)
        : 'N/A',
  },
  {
    title: 'Active Gateways',
    value: data.activeGateways != null ? data.activeGateways.toString() : 'N/A',
    group: 'Operators',
  },
  {
    title: 'Current Operator',
    value: data.currentOperator ?? 'N/A',
    group: 'Operators',
  },
  {
    title: 'Next Operator',
    value: data.nextOperator ?? 'N/A',
    group: 'Operators',
  },
  {
    title: 'L2 Reorgs',
    value: data.l2Reorgs != null ? data.l2Reorgs.toString() : 'N/A',
  },
  {
    title: 'Slashing Events',
    value: data.slashings != null ? data.slashings.toString() : 'N/A',
  },
  {
    title: 'Forced Inclusions',
    value:
      data.forcedInclusions != null ? data.forcedInclusions.toString() : 'N/A',
  },
  {
    title: 'L2 Head Block',
    value: data.l2Block != null ? data.l2Block.toLocaleString() : 'N/A',
  },
  {
    title: 'L1 Head Block',
    value: data.l1Block != null ? data.l1Block.toLocaleString() : 'N/A',
  },
];

export const hasBadRequest = (results: RequestResult<unknown>[]): boolean =>
  results.some((r) => r.badRequest);
