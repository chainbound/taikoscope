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
import { fetchBatchFeeComponents } from '../services/apiService';
import { TimeRange, BatchFeeComponent } from '../types';
import { rangeToHours } from '../utils/timeRange';
import { formatEth } from '../utils';

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
  const { data: feeRes } = useSWR(
    ['batchFeeComponents', timeRange, address],
    () => fetchBatchFeeComponents(timeRange, address),
  );
  const feeData: BatchFeeComponent[] | null = feeRes?.data ?? null;
  const { data: ethPrice = 0, error: ethPriceError } = useEthPrice();

  if (!feeData || feeData.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }

  const hours = rangeToHours(timeRange);
  const HOURS_IN_MONTH = 30 * 24;
  const costPerBatchUsd =
    ((cloudCost + proverCost) / HOURS_IN_MONTH) * (hours / feeData.length);
  const costPerBatchEth = ethPrice ? costPerBatchUsd / ethPrice : 0;

  const data = feeData.map((b) => {
    const revenueEth = (b.priority + b.base - (b.l1Cost ?? 0)) / 1e18;
    const proveEth = (b.amortizedProveCost ?? 0) / 1e18;
    const verifyEth = (b.amortizedVerifyCost ?? 0) / 1e18;
    const profitEth = revenueEth - (costPerBatchEth + proveEth + verifyEth);
    const profitUsd = profitEth * ethPrice;
    return { batch: b.batch, profitEth, profitUsd };
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
            tickFormatter={(v: number) => formatEth(v * 1e18)}
            label={{
              value: ' (ETH)',
              angle: -90,
              position: 'insideLeft',
              offset: -16,
              fontSize: 10,
              fill: '#666666',
            }}
          />
          <Tooltip
            labelFormatter={(v: number) => `Batch ${v}`}
            formatter={(value: number, _name: string, { payload }: any) =>
              [`${formatEth(value * 1e18)} ($${payload.profitUsd.toFixed(2)})`, 'Profit']
            }
            contentStyle={{
              backgroundColor: 'rgba(255,255,255,0.8)',
              borderColor: '#8884d8',
            }}
            labelStyle={{ color: '#333' }}
          />
          <Line
            type="monotone"
            dataKey="profitEth"
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
