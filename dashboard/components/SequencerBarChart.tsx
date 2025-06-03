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
import { PieChartDataItem } from '../types';
import { TAIKO_PINK } from '../theme';

interface SequencerBarChartProps {
  data: PieChartDataItem[];
}

export const SequencerBarChart: React.FC<SequencerBarChartProps> = ({ data }) => {
  if (!data || data.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }
  return (
    <ResponsiveContainer width="100%" height="100%">
      <BarChart data={data} margin={{ top: 5, right: 30, left: 20, bottom: 40 }}>
        <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
        <XAxis
          dataKey="name"
          stroke="#666666"
          fontSize={12}
          label={{
            value: 'Sequencer',
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
          domain={[0, 'auto']}
          allowDecimals={false}
          tickFormatter={(v: number) => v.toLocaleString()}
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
          formatter={(value: number) => [value.toLocaleString(), 'blocks']}
          contentStyle={{
            backgroundColor: 'rgba(255, 255, 255, 0.8)',
            borderColor: TAIKO_PINK,
          }}
          labelStyle={{ color: '#333' }}
        />
        <Bar dataKey="value" fill={TAIKO_PINK} name="Blocks" />
      </BarChart>
    </ResponsiveContainer>
  );
};
