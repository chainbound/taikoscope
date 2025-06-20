import React from 'react';
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
import { L2ReorgEvent } from '../types';
import { TAIKO_PINK } from '../theme';
import { formatDateTime } from '../utils';

interface ReorgDepthChartProps {
  data: L2ReorgEvent[];
}

const ReorgDepthChartComponent: React.FC<ReorgDepthChartProps> = ({ data }) => {
  if (!data || data.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }

  const isMobile = useIsMobile();

  return (
    <ResponsiveContainer width="100%" height="100%">
      <BarChart
        data={data}
        margin={{
          top: 5,
          right: isMobile ? 10 : 20,
          left: isMobile ? 10 : 20,
          bottom: 40,
        }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
        <XAxis
          dataKey="timestamp"
          tickFormatter={(v: number) => formatDateTime(v)}
          stroke="#666666"
          fontSize={12}
          label={{
            value: 'Time',
            position: 'insideBottom',
            offset: -35,
            fontSize: 10,
            fill: '#666666',
          }}
          padding={{ left: 10, right: 10 }}
        />
        <YAxis
          stroke="#666666"
          fontSize={12}
          domain={[0, 'dataMax']}
          label={{
            value: 'Depth',
            angle: -90,
            position: 'insideLeft',
            offset: -16,
            fontSize: 10,
            fill: '#666666',
          }}
        />
        <Tooltip
          labelFormatter={(label: number) => formatDateTime(label)}
          formatter={(value: number) => [value.toString(), 'depth']}
          contentStyle={{
            backgroundColor: 'rgba(255, 255, 255, 0.8)',
            borderColor: TAIKO_PINK,
          }}
          labelStyle={{ color: '#333' }}
        />
        <Bar dataKey="depth" fill={TAIKO_PINK} name="Depth" />
      </BarChart>
    </ResponsiveContainer>
  );
};

export const ReorgDepthChart = React.memo(ReorgDepthChartComponent);
