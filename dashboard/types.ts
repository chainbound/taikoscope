export type TimeRange = '1h' | '24h' | '7d';

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
