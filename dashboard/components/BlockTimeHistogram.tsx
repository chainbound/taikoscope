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
import { formatDecimal, shouldShowMinutes } from '../utils';

interface BlockTimeHistogramProps {
  data: TimeSeriesData[];
  barColor: string;
}

export const BlockTimeHistogram: React.FC<BlockTimeHistogramProps> = ({
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

  const histogramData = useMemo(() => {
    const intervals = data.map((d) => d.timestamp);
    const min = Math.min(...intervals);
    const max = Math.max(...intervals);
    const binCount = 20;
    const binSize = (max - min) / binCount || 1;
    const bins = Array.from({ length: binCount }, (_, i) => ({
      start: min + i * binSize,
      end: min + (i + 1) * binSize,
      count: 0,
    }));
    intervals.forEach((v) => {
      const idx = Math.min(Math.floor((v - min) / binSize), binCount - 1);
      bins[idx].count += 1;
    });
    return bins;
  }, [data]);

  const formatValue = (ms: number) =>
    showMinutes ? formatDecimal(ms / 60000) : formatDecimal(ms / 1000);

  return (
    <ResponsiveContainer width="100%" height="100%">
      <BarChart
        data={histogramData}
        margin={{ top: 5, right: 70, left: 20, bottom: 40 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
        <XAxis
          dataKey="start"
          tickFormatter={(v: number) => formatValue(v)}
          stroke="#666666"
          fontSize={12}
          label={{
            value: showMinutes ? 'Interval (minutes)' : 'Interval (seconds)',
            position: 'insideBottom',
            offset: -35,
            fontSize: 10,
            fill: '#666666',
          }}
        />
        <YAxis
          stroke="#666666"
          fontSize={12}
          domain={[0, 'dataMax']}
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
          labelFormatter={(value: number) => {
            const bin = histogramData.find((b) => b.start === value);
            if (!bin) return '';
            return `${formatValue(bin.start)} - ${formatValue(bin.end)}`;
          }}
          formatter={(value: number) => [value.toString(), 'blocks']}
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
