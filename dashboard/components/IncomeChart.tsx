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
import { useEthPrice } from '../services/priceService';
import { fetchBatchEconomics } from '../services/apiService';
import { TimeRange } from '../types';

interface IncomeChartProps {
  timeRange: TimeRange;
  address?: string;
}

export const IncomeChart: React.FC<IncomeChartProps> = ({
  timeRange,
  address,
}) => {
  const { data: batchRes } = useSWR(['batchEconomics', timeRange, address], () =>
    fetchBatchEconomics(timeRange, address),
  );
  const batchData = batchRes?.data?.batches ?? null;
  const { data: ethPrice = 0, error: ethPriceError } = useEthPrice();

  if (!batchData || batchData.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }

  const data = batchData.map((b) => {
    const revenueEth = b.total_revenue / 1e18;
    const income = revenueEth * ethPrice;
    return { batch_id: b.batch_id, income };
  });

  return (
    <>
      {ethPriceError && (
        <div className="text-red-500 text-xs mb-1">ETH price unavailable</div>
      )}
      <ResponsiveContainer width="100%" height={240}>
        <LineChart
          data={data}
          margin={{ top: 5, right: 20, left: 20, bottom: 40 }}
        >
          <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
          <XAxis
            dataKey="batch_id"
            stroke="#666666"
            fontSize={12}
            label={{
              value: 'Batch ID',
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
              value: 'Income (USD)',
              angle: -90,
              position: 'insideLeft',
              offset: -16,
              fontSize: 10,
              fill: '#666666',
            }}
          />
          <Tooltip
            labelFormatter={(v: number) => `Batch ${v}`}
            formatter={(value: number) => [`$${value.toFixed(2)}`, 'Income']}
            contentStyle={{
              backgroundColor: 'rgba(255,255,255,0.8)',
              borderColor: '#82ca9d',
            }}
            labelStyle={{ color: '#333' }}
          />
          <Line
            type="monotone"
            dataKey="income"
            stroke="#82ca9d"
            strokeWidth={2}
            dot={false}
            name="Income"
          />
        </LineChart>
      </ResponsiveContainer>
    </>
  );
};
