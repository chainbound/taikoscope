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
import { formatEth } from '../utils';
import { SEQUENCER_BASE_FEE_RATIO } from '../utils/profit';
import { fetchL2FeesComponents } from '../services/apiService';
import { TimeRange, BatchFeeComponent } from '../types';

interface RevenueChartProps {
  timeRange: TimeRange;
  address?: string;
}

export const RevenueChart: React.FC<RevenueChartProps> = ({
  timeRange,
  address,
}) => {
  const { data: feeRes } = useSWR(
    ['l2FeesComponents', timeRange, address],
    () => fetchL2FeesComponents(timeRange),
  );
  const feeData: BatchFeeComponent[] | null =
    feeRes?.data?.batches
      ?.filter((b) => !address || b.sequencer === address)
      .map((b) => ({
        batch: b.batch_id,
        txHash: b.l1_tx_hash,
        sequencer: b.sequencer,
        priority: b.priority_fee,
        base: b.base_fee,
        l1Cost: b.l1_data_cost,
        proveCost: b.prove_cost,
      })) ?? null;
  const { data: ethPrice = 0 } = useEthPrice();

  if (!feeData || feeData.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }

  const data = feeData.map((b) => {
    const revenueEth = (b.priority + b.base * SEQUENCER_BASE_FEE_RATIO) / 1e9;
    const revenueUsd = revenueEth * ethPrice;
    return { batch: b.batch, revenueEth, revenueUsd };
  });

  return (
    <>
      {/* Continue rendering; USD values may be zero when price missing */}
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
            tickFormatter={(v: number) => formatEth(v * 1e9, 3)}
            label={{
              value: 'Revenue (ETH)',
              angle: -90,
              position: 'insideLeft',
              offset: -16,
              fontSize: 10,
              fill: '#666666',
            }}
          />
          <Tooltip
            labelFormatter={(v: number) => `Batch ${v}`}
            formatter={(value: number, _name: string, { payload }: Payload<number, 'Revenue'>) =>
              [`${formatEth(value * 1e9, 3)} ($${payload.revenueUsd.toFixed(3)})`, 'Revenue']
            }
            contentStyle={{
              backgroundColor: 'var(--chart-tooltip-bg)',
              borderColor: '#4E79A7',
            }}
            labelStyle={{ color: 'var(--chart-tooltip-label)' }}
          />
          <Line
            type="monotone"
            dataKey="revenueEth"
            stroke="#4E79A7"
            strokeWidth={2}
            dot={false}
            name="Revenue"
          />
        </LineChart>
      </ResponsiveContainer>
    </>
  );
};
