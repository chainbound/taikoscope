import React, { useMemo } from 'react';
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

interface BlockTimeDistributionChartProps {
  data: TimeSeriesData[];
  barColor: string;
}

const BlockTimeDistributionChartComponent: React.FC<BlockTimeDistributionChartProps> = ({
  data,
  barColor,
}) => {
  if (!data || data.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }

  const showMinutes = shouldShowMinutes(data);

  const distributionData = useMemo(() => {
    const times = data.map((d) => d.timestamp);
    const min = Math.min(...times);
    const max = Math.max(...times);
    if (min === max) {
      return [{ interval: min, count: times.length }];
    }
    const binCount = 20;
    const binSize = (max - min) / binCount;
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

  return (
    <ResponsiveContainer width="100%" height="100%">
      <BarChart data={distributionData} margin={{ top: 5, right: 70, left: 80, bottom: 40 }}>
        <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
        <XAxis
          dataKey="interval"
          tickFormatter={(v: number) => formatInterval(v, showMinutes)}
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
          labelFormatter={(label: number) => formatInterval(label, showMinutes)}
          formatter={(value: number) => [value.toLocaleString(), 'blocks']}
          contentStyle={{
            backgroundColor: 'rgba(255, 255, 255, 0.8)',
            borderColor: barColor,
          }}
          labelStyle={{ color: '#333' }}
        />
        <Bar dataKey="count" fill={barColor} name="Blocks" />
      </BarChart>
    </ResponsiveContainer>
  );
};

export const BlockTimeDistributionChart = React.memo(BlockTimeDistributionChartComponent);
