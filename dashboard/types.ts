export type TimeRange = '15m' | '1h' | '24h';

export interface TimeSeriesData {
  timestamp: number; // Unix timestamp (ms)
  value: number;
  name?: string; // For line charts with 'name' on x-axis (like batchId)
}

export interface PieChartDataItem {
  name: string;
  value: number;
  fill?: string; // Color fill is now optional, can be assigned by the chart component
}

import type { ReactNode } from 'react';

export interface MetricData {
  title: ReactNode;
  value: string;
  unit?: string; // e.g., '1h', '24h', or specific units like 'ms'
  description?: ReactNode;
  group?: string;
}

export interface L2ReorgEvent {
  l2_block_number: number;
  depth: number;
  timestamp: number;
}

export interface MissedBlockProposal {
  slot: number;
}

export interface SlashingEvent {
  l1_block_number: number;
  validator_addr: number[];
}

export interface ForcedInclusionEvent {
  blob_hash: number[];
}

export interface ErrorResponse {
  type: string;
  title: string;
  status: number;
  detail: string;
}
