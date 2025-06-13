import React from 'react';
import { type MetricData } from '../types';
import { formatSeconds, formatDecimal, formatEth } from '../utils';
import { getSequencerName } from '../sequencerConfig';

export interface MetricInputData {
  avgTps: number | null;
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
  priorityFee: number | null;
  baseFee: number | null;
}

export const createMetrics = (data: MetricInputData): MetricData[] => [
  {
    title: 'Avg. L2 TPS',
    value: data.avgTps != null ? formatDecimal(data.avgTps) : 'N/A',
    group: 'Network Performance',
  },
  {
    title: 'L2 Block Cadence',
    value:
      data.l2Cadence != null ? formatSeconds(data.l2Cadence / 1000) : 'N/A',
    group: 'Network Performance',
  },
  {
    title: 'Batch Posting Cadence',
    value:
      data.batchCadence != null
        ? formatSeconds(data.batchCadence / 1000)
        : 'N/A',
    group: 'Network Performance',
  },
  {
    title: 'Avg. Prove Time',
    value:
      data.avgProve != null && data.avgProve > 0
        ? formatSeconds(data.avgProve / 1000)
        : 'N/A',
    group: 'Network Health',
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
    group: 'Network Health',
  },
  {
    title: 'Active Sequencers',
    value: data.activeGateways != null ? data.activeGateways.toString() : 'N/A',
    group: 'Sequencers',
  },
  {
    title: 'Current Sequencer',
    value:
      data.currentOperator != null
        ? (() => {
            const name = getSequencerName(data.currentOperator);
            return name === 'Unknown' ? data.currentOperator : name;
          })()
        : 'N/A',
    group: 'Sequencers',
  },
  {
    title: 'Next Sequencer',
    value:
      data.nextOperator != null
        ? (() => {
            const name = getSequencerName(data.nextOperator);
            return name === 'Unknown' ? data.nextOperator : name;
          })()
        : 'N/A',
    group: 'Sequencers',
  },
  {
    title: 'L2 Reorgs',
    value: data.l2Reorgs != null ? data.l2Reorgs.toString() : 'N/A',
    group: 'Network Health',
  },
  {
    title: 'Slashing Events',
    value: data.slashings != null ? data.slashings.toString() : 'N/A',
    group: 'Network Health',
  },
  {
    title: 'Forced Inclusions',
    value:
      data.forcedInclusions != null ? data.forcedInclusions.toString() : 'N/A',
    group: 'Network Health',
  },
  {
    title: 'Priority Fee',
    value: data.priorityFee != null ? formatEth(data.priorityFee) : 'N/A',
    group: 'Network Economics',
  },
  {
    title: 'Base Fee',
    value: data.baseFee != null ? formatEth(data.baseFee) : 'N/A',
    group: 'Network Economics',
  },
  {
    title: 'L2 Block',
    value: data.l2Block != null ? data.l2Block.toLocaleString() : 'N/A',
    group: 'Block Information',
  },
  {
    title: 'L1 Block',
    value: data.l1Block != null ? data.l1Block.toLocaleString() : 'N/A',
    group: 'Block Information',
  },
];
