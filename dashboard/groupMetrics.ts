export const GROUP_ORDER = [
  'Network Performance',
  'Network Health',
  'Operators',
  'Other',
];

import type { MetricData } from './types';

export const groupMetrics = (
  metrics: MetricData[],
): Record<string, MetricData[]> =>
  metrics.reduce<Record<string, MetricData[]>>((acc, m) => {
    const group = m.group ?? 'Other';
    if (!acc[group]) acc[group] = [];
    acc[group].push(m);
    return acc;
  }, {});

