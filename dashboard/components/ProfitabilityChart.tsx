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
import { fetchL2FeesComponents } from '../services/apiService';
import { TimeRange, BatchFeeComponent } from '../types';
import { rangeToHours } from '../utils/timeRange';
import { formatEth } from '../utils';
import { calculateProfit } from '../utils/profit';

interface ProfitabilityChartProps {
  timeRange: TimeRange;
  cloudCost: number;
  proverCost: number;
  address?: string;
  /** Total number of sequencers for scaling network-wide costs */
  totalSequencers?: number;
}

export const ProfitabilityChart: React.FC<ProfitabilityChartProps> = ({
  timeRange,
  cloudCost,
  proverCost,
  address,
  totalSequencers,
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
        amortizedProveCost: b.amortized_prove_cost,
      })) ?? null;
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
  const seqCount = address ? 1 : Math.max(1, totalSequencers ?? 1);
  const costPerBatchUsd =
    ((cloudCost + proverCost) * seqCount) / HOURS_IN_MONTH * (hours / feeData.length);

  const data = feeData.map((b) => {
    const { profitEth, profitUsd } = calculateProfit({
      priorityFee: b.priority,
      baseFee: b.base,
      l1DataCost: b.l1Cost ?? 0,
      proveCost: b.amortizedProveCost ?? 0,

      hardwareCostUsd: costPerBatchUsd,
      ethPrice,
    });
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
            tickFormatter={(v: number) => formatEth(v * 1e9, 3)}
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
            formatter={(value: number, _name: string, { payload }: Payload<number, "Profit">) =>
              [`${formatEth(value * 1e9, 3)} ($${payload.profitUsd.toFixed(3)})`, 'Profit']
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
