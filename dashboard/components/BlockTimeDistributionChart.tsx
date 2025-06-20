import React, { useMemo } from 'react';
import { useIsMobile } from '../hooks/useIsMobile';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import { TimeSeriesData } from '../types';
import { formatInterval, shouldShowMinutes } from '../utils';

// Constants for histogram configuration
const MIN_BIN_COUNT = 5;
const MAX_BIN_COUNT = 20;
const MIN_REASONABLE_BLOCK_TIME_MS = 0;
const MAX_REASONABLE_BLOCK_TIME_MS = 24 * 60 * 60 * 1000; // 24 hours in milliseconds
const BASE_MARGINS = { top: 5, right: 20, left: 20, bottom: 40 };

interface BlockTimeDistributionChartProps {
  data: TimeSeriesData[];
  barColor: string;
}

const BlockTimeDistributionChartComponent: React.FC<
  BlockTimeDistributionChartProps
> = ({ data, barColor }) => {
  if (!data || data.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }

  const showMinutes = shouldShowMinutes(data);
  const isMobile = useIsMobile();

  const distributionData = useMemo(() => {
    // Extract block times (timestamps) and filter for reasonable bounds
    const times = data
      .map((d) => d.timestamp)
      .filter(
        (time) =>
          time >= MIN_REASONABLE_BLOCK_TIME_MS &&
          time <= MAX_REASONABLE_BLOCK_TIME_MS,
      );

    if (times.length === 0) {
      return [];
    }

    const min = Math.min(...times);
    const max = Math.max(...times);

    if (min === max) {
      return [{ interval: min, count: times.length }];
    }

    // Adaptive bin count based on data size
    const binCount = Math.min(
      MAX_BIN_COUNT,
      Math.max(MIN_BIN_COUNT, Math.floor(Math.sqrt(times.length))),
    );

    const binSize = (max - min) / binCount;
    const EPSILON = 1e-10; // Small constant to prevent floating point issues

    if (binSize < EPSILON) {
      return [{ interval: (min + max) / 2, count: times.length }];
    }
    const bins = Array.from({ length: binCount }, (_, i) => ({
      interval: min + (i + 0.5) * binSize,
      count: 0,
    }));

    times.forEach((t) => {
      const idx = Math.min(Math.floor((t - min) / binSize), binCount - 1);
      bins[idx].count += 1;
    });

    return bins;
  }, [data]);

  if (distributionData.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No valid block time data available
      </div>
    );
  }

  const CHART_MARGINS = {
    top: BASE_MARGINS.top,
    right: isMobile ? 10 : BASE_MARGINS.right,
    left: isMobile ? 10 : BASE_MARGINS.left,
    bottom: BASE_MARGINS.bottom,
  };

  return (
    <ResponsiveContainer width="100%" height="100%">
      <BarChart data={distributionData} margin={CHART_MARGINS}>
        <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
        <XAxis
          dataKey="interval"
          tickFormatter={(v: number) => formatInterval(v, false, showMinutes)}
          stroke="#666666"
          fontSize={12}
          label={{
            value: showMinutes ? 'Minutes' : 'Seconds',
            position: 'insideBottom',
            offset: -35,
            fontSize: 10,
            fill: '#666666',
          }}
        />
        <YAxis
          stroke="#666666"
          fontSize={12}
          allowDecimals={false}
          label={{
            value: 'Blocks',
            angle: -90,
            position: 'insideLeft',
            offset: -16,
            fontSize: 10,
            fill: '#666666',
          }}
        />
        <Tooltip
          labelFormatter={(label: number) =>
            formatInterval(label, false, showMinutes)
          }
          formatter={(value: number) => [value.toLocaleString(), 'blocks']}
          contentStyle={{
            backgroundColor: 'rgba(255, 255, 255, 0.8)',
            borderColor: barColor,
            borderWidth: 1,
            borderRadius: 4,
          }}
          labelStyle={{ color: '#333' }}
        />
        <Bar dataKey="count" fill={barColor} name="Blocks" />
      </BarChart>
    </ResponsiveContainer>
  );
};

export const BlockTimeDistributionChart = React.memo(
  BlockTimeDistributionChartComponent,
);
