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
import { rangeToHours } from '../utils/timeRange';

interface ProfitabilityChartProps {
  timeRange: TimeRange;
  cloudCost: number;
  proverCost: number;
  address?: string;
}

export const ProfitabilityChart: React.FC<ProfitabilityChartProps> = ({
  timeRange,
  cloudCost,
  proverCost,
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

  const hours = rangeToHours(timeRange);
  const HOURS_IN_MONTH = 30 * 24;
  const totalCost = ((cloudCost + proverCost) / HOURS_IN_MONTH) * hours;
  const costPerBatch = totalCost / batchData.length;

  const data = batchData.map((b) => {
    const revenueEth = b.net_revenue / 1e18;
    const profit = revenueEth * ethPrice - costPerBatch;
    return { batch_id: b.batch_id, profit };
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
              value: 'Profit (USD)',
              angle: -90,
              position: 'insideLeft',
              offset: -16,
              fontSize: 10,
              fill: '#666666',
            }}
          />
          <Tooltip
            labelFormatter={(v: number) => `Batch ${v}`}
            formatter={(value: number) => [`$${value.toFixed(2)}`, 'Profit']}
            contentStyle={{
              backgroundColor: 'rgba(255,255,255,0.8)',
              borderColor: '#8884d8',
            }}
            labelStyle={{ color: '#333' }}
          />
          <Line
            type="monotone"
            dataKey="profit"
            stroke="#8884d8"
            strokeWidth={2}
            dot={false}
            name="Profit"
          />
        </LineChart>
      </ResponsiveContainer>
    </>
  );
};
