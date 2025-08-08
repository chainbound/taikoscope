import React from 'react';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import { useIsMobile } from '../hooks/useIsMobile';
import { L2ReorgEvent } from '../types';
// brand color via CSS variable
import { formatDateTime } from '../utils';

interface ReorgDepthChartProps {
  data: L2ReorgEvent[];
}

const ReorgDepthChartComponent: React.FC<ReorgDepthChartProps> = ({ data }) => {
  const isMobile = useIsMobile();
  if (!data || data.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }

  return (
    <ResponsiveContainer width="100%" height="100%">
      <BarChart
        data={data}
        margin={{ top: 5, right: 20, left: 20, bottom: 40 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="var(--chart-grid)" />
        <XAxis
          dataKey="timestamp"
          tickFormatter={(v: number) => formatDateTime(v)}
          stroke="var(--chart-tick)"
          fontSize={12}
          label={{
            value: 'Time',
            position: 'insideBottom',
            offset: -35,
            fontSize: 10,
            fill: 'var(--chart-tick)',
          }}
          padding={{ left: isMobile ? 5 : 10, right: isMobile ? 5 : 10 }}
        />
        <YAxis
          stroke="var(--chart-tick)"
          fontSize={12}
          domain={[0, 'dataMax']}
          label={{
            value: 'Depth',
            angle: -90,
            position: 'insideLeft',
            offset: -16,
            fontSize: 10,
            fill: 'var(--chart-tick)',
          }}
        />
        <Tooltip
          labelFormatter={(label: number) => formatDateTime(label)}
          formatter={(value: number) => [value.toString(), 'depth']}
          contentStyle={{
            backgroundColor: 'var(--chart-tooltip-bg)',
            borderColor: 'var(--color-brand)',
          }}
          labelStyle={{ color: 'var(--chart-tooltip-label)' }}
        />
        <Bar dataKey="depth" fill={'var(--color-brand)'} name="Depth" />
      </BarChart>
    </ResponsiveContainer>
  );
};

export const ReorgDepthChart = React.memo(ReorgDepthChartComponent);
