import React from 'react';
import useSWR from 'swr';
import { fetchBatchProfits, fetchBatchEconomics } from '../services/apiService';
import { useEthPrice } from '../services/priceService';
import { TimeRange } from '../types';
import { rangeToHours } from '../utils/timeRange';

interface BatchProfitTablesProps {
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

export const BatchProfitTables: React.FC<BatchProfitTablesProps> = ({
  timeRange,
  cloudCost,
  proverCost,
  address,
}) => {
  const { data: ethPrice = 0 } = useEthPrice();
  const { data: topRes } = useSWR(['topBatchProfits', timeRange, address], () =>
    fetchBatchProfits(timeRange, 'desc', 5, address),
  );
  const { data: bottomRes } = useSWR(['bottomBatchProfits', timeRange, address], () =>
    fetchBatchProfits(timeRange, 'asc', 5, address),
  );
  const { data: batchEconomicsRes } = useSWR(['batchEconomics', timeRange, address], () =>
    fetchBatchEconomics(timeRange, address),
  );
  const batchCount = batchEconomicsRes?.data?.batches.length ?? 0;
  const HOURS_IN_MONTH = 30 * 24;
  const hours = rangeToHours(timeRange);
  const costPerBatch =
    batchCount > 0 ? ((cloudCost + proverCost) / HOURS_IN_MONTH) * hours / batchCount : 0;

  const calcProfit = (wei: number) => (wei / 1e18) * ethPrice - costPerBatch;

  const renderTable = (title: string, items: { batch_id: number; profit: number }[] | null) => (
    <div>
      <h3 className="text-lg font-semibold mb-2">{title}</h3>
      <div className="overflow-x-auto">
        <table className="min-w-full border border-gray-200 dark:border-gray-700 divide-y divide-gray-200 dark:divide-gray-700 bg-white dark:bg-gray-800">
          <thead>
            <tr>
              <th className="px-2 py-1 text-left">Batch</th>
              <th className="px-2 py-1 text-left">Profit (USD)</th>
            </tr>
          </thead>
          <tbody>
            {items?.map((b) => (
              <tr key={b.batch_id} className="border-t border-gray-200 dark:border-gray-700">
                <td className="px-2 py-1">{b.batch_id}</td>
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
      {renderTable('Top 5 Profitable Batches', topRes?.data?.batches ?? null)}
      {renderTable('Least 5 Profitable Batches', bottomRes?.data?.batches ?? null)}
    </div>
  );
};

// Legacy export for backward compatibility
export { BatchProfitTables as BlockProfitTables };
