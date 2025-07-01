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
import type { Payload } from 'recharts/types/component/DefaultTooltipContent';
import useSWR from 'swr';
import { useEthPrice } from '../services/priceService';
import { fetchBatchFeeComponents } from '../services/apiService';
import { TimeRange, BatchFeeComponent } from '../types';
import { rangeToHours } from '../utils/timeRange';
import { formatEth } from '../utils';

interface EconomicsChartProps {
  timeRange: TimeRange;
  cloudCost: number;
  proverCost: number;
  address?: string;
}

export const EconomicsChart: React.FC<EconomicsChartProps> = ({
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
  const baseCostUsd = ((cloudCost + proverCost) / HOURS_IN_MONTH) * hours;
  const baseCostPerBatchUsd = baseCostUsd / feeData.length;
  const baseCostPerBatchEth = ethPrice ? baseCostPerBatchUsd / ethPrice : 0;

  const data = feeData.map((b) => {
    const revenueEth = b.priority + b.base;
    const proveEth = b.amortizedProveCost ?? 0;
    const verifyEth = 0;
    const costEth = baseCostPerBatchEth + proveEth + verifyEth + (b.l1Cost ?? 0);
    const profitEth = revenueEth - costEth;
    const revenueUsd = revenueEth * ethPrice;
    const costUsd = costEth * ethPrice;
    const profitUsd = profitEth * ethPrice;
    return {
      batch: b.batch,
      sequencer: b.sequencer,
      revenueEth,
      costEth,
      profitEth,
      revenueUsd,
      costUsd,
      profitUsd,
    };
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
            tickFormatter={(v: number) => formatEth(v * 1e18, 3)}
            label={{
              value: 'ETH',
              angle: -90,
              position: 'insideLeft',
              offset: -16,
              fontSize: 10,
              fill: '#666666',
            }}
          />
          <Tooltip
            labelFormatter={(v: number, payload) => {
              const seq = payload?.[0]?.payload?.sequencer as string | undefined;
              return seq ? `Batch ${v} - ${seq}` : `Batch ${v}`;
            }}
            formatter={(value: number, name: string, { payload }: Payload<number, string>) => {
              if (name === 'Revenue')
                return [
                  `${formatEth(value * 1e18, 3)} ($${payload.revenueUsd.toFixed(3)})`,
                  name,
                ];
              if (name === 'Cost')
                return [
                  `${formatEth(value * 1e18, 3)} ($${payload.costUsd.toFixed(3)})`,
                  name,
                ];
              return [
                `${formatEth(value * 1e18, 3)} ($${payload.profitUsd.toFixed(3)})`,
                name,
              ];
            }}
            contentStyle={{
              backgroundColor: 'rgba(255,255,255,0.8)',
              borderColor: '#8884d8',
            }}
            labelStyle={{ color: '#333' }}
          />
          <Line
            type="monotone"
            dataKey="revenueEth"
            stroke="#4E79A7"
            strokeWidth={2}
            dot={false}
            name="Revenue"
          />
          <Line
            type="monotone"
            dataKey="costEth"
            stroke="#E573B5"
            strokeWidth={2}
            dot={false}
            name="Cost"
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
