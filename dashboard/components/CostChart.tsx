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
import { fetchBatchFeeComponents } from '../services/apiService';
import { TimeRange, BatchFeeComponent } from '../types';
import { rangeToHours } from '../utils/timeRange';
import { useEthPrice } from '../services/priceService';

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
  const { data: feeRes } = useSWR(
    ['batchFeeComponents', timeRange, address],
    () => fetchBatchFeeComponents(timeRange, address),
  );
  const { data: ethPrice = 0, error: ethPriceError } = useEthPrice();
  const feeData: BatchFeeComponent[] | null = feeRes?.data ?? null;

  if (!feeData || feeData.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }

  const hours = rangeToHours(timeRange);
  const HOURS_IN_MONTH = 30 * 24;
  const baseCost = ((cloudCost + proverCost) / HOURS_IN_MONTH) * hours;
  const baseCostPerBatch = baseCost / feeData.length;

  const data = feeData.map((b) => {
    const l1CostUsd = ((b.l1Cost ?? 0) / 1e18) * ethPrice;
    return { batch: b.batch, cost: baseCostPerBatch + l1CostUsd };
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
          dataKey="batch"
          stroke="#666666"
          fontSize={12}
          label={{
            value: 'Batch',
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
          labelFormatter={(v: number) => `Batch ${v}`}
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
    </>
  );
};
