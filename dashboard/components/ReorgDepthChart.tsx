import React from 'react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from 'recharts';
import { L2ReorgEvent } from '../types';

interface ReorgDepthChartProps {
  data: L2ReorgEvent[];
}

const TAIKO_PINK = '#e81899';

export const ReorgDepthChart: React.FC<ReorgDepthChartProps> = ({ data }) => {
  if (!data || data.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }

  return (
    <ResponsiveContainer width="100%" height="100%">
      <LineChart
        data={data}
        margin={{ top: 5, right: 30, left: 20, bottom: 50 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
        <XAxis
          dataKey="l2_block_number"
          tickFormatter={(v: number) => v.toLocaleString()}
          stroke="#666666"
          fontSize={12}
          label={{
            value: 'Block Number',
            position: 'insideBottom',
            offset: -10,
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
          labelFormatter={(label: number) => `Block ${label.toLocaleString()}`}
          formatter={(value: number) => [value.toString(), 'depth']}
          contentStyle={{
            backgroundColor: 'rgba(255, 255, 255, 0.8)',
            borderColor: TAIKO_PINK,
          }}
          labelStyle={{ color: '#333' }}
        />
        <Legend
          verticalAlign="bottom"
          align="right"
          wrapperStyle={{ right: 20, bottom: 0 }}
        />
        <Line
          type="monotone"
          dataKey="depth"
          stroke={TAIKO_PINK}
          strokeWidth={2}
          dot={false}
          activeDot={data.length <= 100 ? { r: 6 } : false}
          name="Depth"
        />
      </LineChart>
    </ResponsiveContainer>
  );
};
