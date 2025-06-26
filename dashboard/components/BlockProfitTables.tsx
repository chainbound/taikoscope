import React from 'react';
import useSWR from 'swr';
import { fetchBatchFeeComponents } from '../services/apiService';
import { useEthPrice } from '../services/priceService';
import { TimeRange } from '../types';
import { rangeToHours } from '../utils/timeRange';
import { formatEth, l1BlockLink } from '../utils';
import { calculateProfit } from '../utils/profit';

interface BlockProfitTablesProps {
  timeRange: TimeRange;
  cloudCost: number;
  proverCost: number;
  address?: string;
}

const formatUsd = (value: number): string => {
  const abs = Math.abs(value);
  if (abs >= 1000) return Math.trunc(value).toLocaleString();
  return value.toLocaleString(undefined, {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
};

export const BlockProfitTables: React.FC<BlockProfitTablesProps> = ({
  timeRange,
  cloudCost,
  proverCost,
  address,
}) => {
  const { data: ethPrice = 0 } = useEthPrice();
  const { data: feeRes } = useSWR(
    ['batchFeeComponents', timeRange, address],
    () => fetchBatchFeeComponents(timeRange, address),
  );
  const batchData = feeRes?.data ?? [];
  const batchCount = batchData.length;
  const HOURS_IN_MONTH = 30 * 24;
  const hours = rangeToHours(timeRange);

  // Calculate operational costs per batch in USD
  const operationalCostPerBatchUsd =
    batchCount > 0
      ? ((cloudCost + proverCost) / HOURS_IN_MONTH) * (hours / batchCount)
      : 0;

  const profits = batchData.map((b) => {
    const { profitEth } = calculateProfit({
      priorityWei: b.priority,
      baseWei: b.base,
      l1CostWei: b.l1Cost ?? 0,
      proveCostWei: b.amortizedProveCost ?? 0,
      verifyCostWei: b.amortizedVerifyCost ?? 0,
      hardwareCostUsd: operationalCostPerBatchUsd,
      ethPrice,
    });

    const profitWei = profitEth * 1e18;

    return {
      batch: b.batch,
      l1Block: b.l1Block,
      sequencer: b.sequencer,
      profit: profitWei, // Store as wei for consistency
      profitEth, // Store ETH value for sorting and display
    };
  });

  const topBatches = [...profits]
    .sort((a, b) => b.profitEth - a.profitEth)
    .slice(0, 5);
  const bottomBatches = [...profits]
    .sort((a, b) => a.profitEth - b.profitEth)
    .slice(0, 5);

  const renderTable = (
    title: string,
    items:
      | { batch: number; l1Block: number; sequencer: string; profit: number; profitEth: number }[]
      | null,
  ) => (
    <div>
      <h3 className="text-lg font-semibold mb-2">{title}</h3>
      <div className="overflow-x-auto">
        <table className="min-w-full border border-gray-200 dark:border-gray-700 divide-y divide-gray-200 dark:divide-gray-700 bg-white dark:bg-gray-800">
          <thead>
            <tr>
              <th className="px-2 py-1 text-left">Batch</th>
              <th className="px-2 py-1 text-left">Sequencer</th>
              <th className="px-2 py-1 text-left">Profit</th>
            </tr>
          </thead>
          <tbody>
            {items?.map((b) => (
              <tr
                key={b.batch}
                className="border-t border-gray-200 dark:border-gray-700"
              >
                <td className="px-2 py-1">
                  {l1BlockLink(b.l1Block ?? 0, b.batch.toLocaleString())}
                </td>
                <td className="px-2 py-1">{b.sequencer}</td>
                <td
                  className="px-2 py-1"
                  title={`$${formatUsd(b.profitEth * ethPrice)}`}
                >
                  {formatEth(b.profit)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );

  return (
    <div className="mt-6 grid grid-cols-1 md:grid-cols-2 gap-4 md:gap-6">
      {renderTable('Top 5 Profitable Batches', topBatches)}
      {renderTable('Least 5 Profitable Batches', bottomBatches)}
    </div>
  );
};
