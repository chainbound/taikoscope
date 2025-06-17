import React from 'react';
import useSWR from 'swr';
import { TimeRange } from '../types';
import {
  fetchSequencerDistribution,
  fetchL2Fees,
} from '../services/apiService';
import { getSequencerAddress } from '../sequencerConfig';
import { useEthPrice } from '../services/priceService';
import { rangeToHours } from '../utils/timeRange';

interface ProfitRankingTableProps {
  timeRange: TimeRange;
  cloudCost: number;
  proverCost: number;
}

const formatProfit = (value: number): string => {
  const abs = Math.abs(value);
  if (abs >= 1000) {
    return Math.trunc(value).toLocaleString();
  }
  return value.toLocaleString(undefined, {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
};

export const ProfitRankingTable: React.FC<ProfitRankingTableProps> = ({
  timeRange,
  cloudCost,
  proverCost,
}) => {
  const { data: distRes } = useSWR(['profitRankingSeq', timeRange], () =>
    fetchSequencerDistribution(timeRange),
  );
  const sequencers = distRes?.data ?? [];

  const { data: ethPrice = 0 } = useEthPrice();

  const { data: feeData } = useSWR(
    sequencers.length ? ['profitRankingFees', timeRange, sequencers] : null,
    async () => {
      const fees = await Promise.all(
        sequencers.map((s) =>
          fetchL2Fees(timeRange, getSequencerAddress(s.name) || ''),
        ),
      );
      return fees.map((f) => f.data);
    },
  );

  if (!feeData) {
    return (
      <div className="flex items-center justify-center h-20 text-gray-500 dark:text-gray-400">
        Loading...
      </div>
    );
  }

  const hours = rangeToHours(timeRange);
  const MONTH_HOURS = 30 * 24;
  const costPerSeq = ((cloudCost + proverCost) / MONTH_HOURS) * hours;

  const rows = sequencers.map((seq, idx) => {
    const fees = feeData[idx];
    if (!fees) {
      return {
        name: seq.name,
        blocks: seq.value,
        profit: null as number | null,
      };
    }
    const revenueEth =
      ((fees.priority_fee ?? 0) +
        (fees.base_fee ?? 0) -
        (fees.l1_data_cost ?? 0)) /
      1e18;
    const profit = revenueEth * ethPrice - costPerSeq;
    return { name: seq.name, blocks: seq.value, profit };
  });

  const sorted = rows.sort(
    (a, b) => (b.profit ?? -Infinity) - (a.profit ?? -Infinity),
  );

  return (
    <div className="mt-6">
      <h3 className="text-lg font-semibold mb-2">Sequencer Profit Ranking</h3>
      <div className="overflow-x-auto">
        <table className="min-w-full border border-gray-200 dark:border-gray-700 divide-y divide-gray-200 dark:divide-gray-700 bg-white dark:bg-gray-800">
          <thead>
            <tr>
              <th className="px-2 py-1 text-left">Sequencer</th>
              <th className="px-2 py-1 text-left">Blocks</th>
              <th className="px-2 py-1 text-left">Profit (USD)</th>
            </tr>
          </thead>
          <tbody>
            {sorted.map((row) => (
              <tr
                key={row.name}
                className="border-t border-gray-200 dark:border-gray-700"
              >
                <td className="px-2 py-1">{row.name}</td>
                <td className="px-2 py-1">{row.blocks.toLocaleString()}</td>
                <td className="px-2 py-1">
                  {row.profit != null ? `$${formatProfit(row.profit)}` : 'N/A'}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
};
