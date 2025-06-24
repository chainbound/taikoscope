import React from 'react';
import useSWR from 'swr';
import { fetchBlockProfits, fetchFeeComponents } from '../services/apiService';
import { useEthPrice } from '../services/priceService';
import { TimeRange } from '../types';
import { rangeToHours } from '../utils/timeRange';

interface BlockProfitTablesProps {
  timeRange: TimeRange;
  cloudCost: number;
  proverCost: number;
  address?: string;
}

const formatUsd = (value: number): string => {
  const abs = Math.abs(value);
  if (abs >= 1000) return Math.trunc(value).toLocaleString();
  return value.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 });
};

export const BlockProfitTables: React.FC<BlockProfitTablesProps> = ({
  timeRange,
  cloudCost,
  proverCost,
  address,
}) => {
  const { data: ethPrice = 0 } = useEthPrice();
  const { data: topRes } = useSWR(['topProfits', timeRange, address], () =>
    fetchBlockProfits(timeRange, 'desc', 5, address),
  );
  const { data: bottomRes } = useSWR(['bottomProfits', timeRange, address], () =>
    fetchBlockProfits(timeRange, 'asc', 5, address),
  );
  const { data: feeRes } = useSWR(['feeComponents', timeRange, address], () =>
    fetchFeeComponents(timeRange, address),
  );
  const blockCount = feeRes?.data?.length ?? 0;
  const HOURS_IN_MONTH = 30 * 24;
  const hours = rangeToHours(timeRange);
  const costPerBlock =
    blockCount > 0 ? ((cloudCost + proverCost) / HOURS_IN_MONTH) * hours / blockCount : 0;

  const calcProfit = (wei: number) => (wei / 1e18) * ethPrice - costPerBlock;

  const renderTable = (title: string, items: { block: number; profit: number }[] | null) => (
    <div>
      <h3 className="text-lg font-semibold mb-2">{title}</h3>
      <div className="overflow-x-auto">
        <table className="min-w-full border border-gray-200 dark:border-gray-700 divide-y divide-gray-200 dark:divide-gray-700 bg-white dark:bg-gray-800">
          <thead>
            <tr>
              <th className="px-2 py-1 text-left">Block</th>
              <th className="px-2 py-1 text-left">Profit (USD)</th>
            </tr>
          </thead>
          <tbody>
            {items?.map((b) => (
              <tr key={b.block} className="border-t border-gray-200 dark:border-gray-700">
                <td className="px-2 py-1">{b.block}</td>
                <td className="px-2 py-1">${formatUsd(calcProfit(b.profit))}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );

  return (
    <div className="mt-6 grid grid-cols-1 md:grid-cols-2 gap-4 md:gap-6">
      {renderTable('Top 5 Profitable Blocks', topRes?.data ?? null)}
      {renderTable('Least 5 Profitable Blocks', bottomRes?.data ?? null)}
    </div>
  );
};
