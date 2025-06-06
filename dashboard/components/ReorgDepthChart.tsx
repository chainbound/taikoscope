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
import { L2ReorgEvent } from '../types';
import { TAIKO_PINK } from '../theme';

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

  const filledData = useMemo(() => {
    const sorted = [...data].sort((a, b) => a.l2_block_number - b.l2_block_number);
    const filled: L2ReorgEvent[] = [];
    if (sorted.length === 0) {
      return filled;
    }

    let expected = sorted[0].l2_block_number;
    for (const event of sorted) {
      while (expected < event.l2_block_number) {
        filled.push({ l2_block_number: expected, depth: 0, timestamp: 0 });
        expected += 1;
      }
      filled.push(event);
      expected = event.l2_block_number + 1;
    }
    return filled;
  }, [data]);

  return (
    <ResponsiveContainer width="100%" height="100%">
      <BarChart
        data={filledData}
        margin={{ top: 5, right: 70, left: 60, bottom: 40 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
        <XAxis
          dataKey="l2_block_number"
          tickFormatter={(v: number) => v.toLocaleString()}
          stroke="#666666"
          fontSize={12}
          label={{
            value: 'L2 Block Number',
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
          labelFormatter={(label: number) => `Block ${label.toLocaleString()}`}
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
