import React from 'react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import useSWR from 'swr';
import { fetchFeeComponents } from '../services/apiService';
import { TimeRange, FeeComponent } from '../types';
import { rangeToHours } from '../utils/timeRange';

interface CostChartProps {
  timeRange: TimeRange;
  cloudCost: number;
  proverCost: number;
  address?: string;
}

export const CostChart: React.FC<CostChartProps> = ({
  timeRange,
  cloudCost,
  proverCost,
  address,
}) => {
  const { data: feeRes } = useSWR(['feeComponents', timeRange, address], () =>
    fetchFeeComponents(timeRange, address),
  );
  const feeData: FeeComponent[] | null = feeRes?.data ?? null;

  if (!feeData || feeData.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }

  const hours = rangeToHours(timeRange);
  const HOURS_IN_MONTH = 30 * 24;
  const totalCost = ((cloudCost + proverCost) / HOURS_IN_MONTH) * hours;
  const costPerBlock = totalCost / feeData.length;

  const data = feeData.map((b) => ({ block: b.block, cost: costPerBlock }));

  return (
    <ResponsiveContainer width="100%" height={240}>
      <LineChart
        data={data}
        margin={{ top: 5, right: 20, left: 20, bottom: 40 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
        <XAxis
          dataKey="block"
          stroke="#666666"
          fontSize={12}
          label={{
            value: 'L2 Block',
            position: 'insideBottom',
            offset: -10,
            fontSize: 10,
            fill: '#666666',
          }}
        />
        <YAxis
          stroke="#666666"
          fontSize={12}
          domain={['auto', 'auto']}
          tickFormatter={(v: number) => `$${v.toLocaleString()}`}
          label={{
            value: 'Cost (USD)',
            angle: -90,
            position: 'insideLeft',
            offset: -16,
            fontSize: 10,
            fill: '#666666',
          }}
        />
        <Tooltip
          labelFormatter={(v: number) => `Block ${v}`}
          formatter={(value: number) => [`$${value.toFixed(2)}`, 'Cost']}
          contentStyle={{
            backgroundColor: 'rgba(255,255,255,0.8)',
            borderColor: '#E573B5',
          }}
          labelStyle={{ color: '#333' }}
        />
        <Line
          type="monotone"
          dataKey="cost"
          stroke="#E573B5"
          strokeWidth={2}
          dot={false}
          name="Cost"
        />
      </LineChart>
    </ResponsiveContainer>
  );
};
