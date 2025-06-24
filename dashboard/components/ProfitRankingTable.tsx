import React from 'react';
import useSWR from 'swr';
import { TimeRange } from '../types';
import {
  fetchSequencerDistribution,
  fetchBatchL2Fees,
} from '../services/apiService';
import * as apiService from '../services/apiService';
import { getSequencerAddress } from '../sequencerConfig';
import { addressLink } from '../utils';
import { useEthPrice } from '../services/priceService';
import { rangeToHours } from '../utils/timeRange';

interface ProfitRankingTableProps {
  timeRange: TimeRange;
  cloudCost: number;
  proverCost: number;
}

const formatUsd = (value: number): string => {
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

  const { data: batchFeeRes } = useSWR(['profitRankingBatchFees', timeRange], () =>
    fetchBatchL2Fees(timeRange),
  );
  const feeDataMap = React.useMemo(() => {
    const map = new Map<string, apiService.BatchSequencerFeeRow>();
    batchFeeRes?.data?.sequencers.forEach((f) => {
      map.set(f.address.toLowerCase(), f);
    });
    return map;
  }, [batchFeeRes]);

  const [sortBy, setSortBy] = React.useState<
    'name' | 'blocks' | 'revenue' | 'cost' | 'profit'
  >('profit');
  const [sortDirection, setSortDirection] = React.useState<'asc' | 'desc'>(
    'desc',
  );

  const hours = rangeToHours(timeRange);
  const MONTH_HOURS = 30 * 24;
  const costPerSeq = ((cloudCost + proverCost) / MONTH_HOURS) * hours;

  const rows = sequencers.map((seq) => {
    const addr = seq.address || getSequencerAddress(seq.name) || '';
    const fees = feeDataMap.get(addr.toLowerCase());
    if (!fees) {
      return {
        name: seq.name,
        address: addr,
        blocks: seq.value,
        revenue: null as number | null,
        cost: costPerSeq,
        profit: null as number | null,
      };
    }
    const revenueEth =
      ((fees.priority_fee ?? 0) +
        (fees.base_fee ?? 0) * 0.75 -
        (fees.l1_data_cost ?? 0)) /
      1e18;
    const revenueUsd = revenueEth * ethPrice;
    const profit = revenueUsd - costPerSeq;
    return {
      name: seq.name,
      address: addr,
      blocks: seq.value,
      revenue: revenueUsd,
      cost: costPerSeq,
      profit,
    };
  });

  const sorted = React.useMemo(() => {
    const data = [...rows];
    data.sort((a, b) => {
      const aVal = a[sortBy];
      const bVal = b[sortBy];
      let cmp = 0;
      // Handle numeric columns including null values
      if (sortBy === 'blocks' || sortBy === 'revenue' || sortBy === 'cost' || sortBy === 'profit') {
        const safeA = (typeof aVal === 'number' ? aVal : null) ?? -Infinity;
        const safeB = (typeof bVal === 'number' ? bVal : null) ?? -Infinity;
        cmp = safeA - safeB;
      } else {
        cmp = String(aVal).localeCompare(String(bVal), undefined, {
          numeric: true,
        });
      }
      return sortDirection === 'desc' ? -cmp : cmp;
    });
    return data;
  }, [rows, sortBy, sortDirection]);

  if (!batchFeeRes) {
    return (
      <div className="flex items-center justify-center h-20 text-gray-500 dark:text-gray-400">
        Loading...
      </div>
    );
  }

  const handleSort = (
    column: 'name' | 'blocks' | 'revenue' | 'cost' | 'profit',
  ) => {
    if (sortBy === column) {
      setSortDirection((d) => (d === 'asc' ? 'desc' : 'asc'));
    } else {
      setSortBy(column);
      setSortDirection('desc');
    }
  };

  return (
    <div className="mt-6">
      <h3 className="text-lg font-semibold mb-2">Sequencer Profit Ranking</h3>
      <div className="overflow-x-auto">
        <table className="min-w-full border border-gray-200 dark:border-gray-700 divide-y divide-gray-200 dark:divide-gray-700 bg-white dark:bg-gray-800">
          <thead>
            <tr>
              <th
                className="px-2 py-1 text-left cursor-pointer select-none"
                onClick={() => handleSort('name')}
              >
                Sequencer
                {sortBy === 'name' && (
                  <span className="ml-1">
                    {sortDirection === 'asc' ? '↑' : '↓'}
                  </span>
                )}
              </th>
              <th
                className="px-2 py-1 text-left cursor-pointer select-none"
                onClick={() => handleSort('blocks')}
              >
                Blocks
                {sortBy === 'blocks' && (
                  <span className="ml-1">
                    {sortDirection === 'asc' ? '↑' : '↓'}
                  </span>
                )}
              </th>
              <th
                className="px-2 py-1 text-left cursor-pointer select-none"
                onClick={() => handleSort('revenue')}
              >
                Revenue (USD)
                {sortBy === 'revenue' && (
                  <span className="ml-1">
                    {sortDirection === 'asc' ? '↑' : '↓'}
                  </span>
                )}
              </th>
              <th
                className="px-2 py-1 text-left cursor-pointer select-none"
                onClick={() => handleSort('cost')}
              >
                Cost (USD)
                {sortBy === 'cost' && (
                  <span className="ml-1">
                    {sortDirection === 'asc' ? '↑' : '↓'}
                  </span>
                )}
              </th>
              <th
                className="px-2 py-1 text-left cursor-pointer select-none"
                onClick={() => handleSort('profit')}
              >
                Profit (USD)
                {sortBy === 'profit' && (
                  <span className="ml-1">
                    {sortDirection === 'asc' ? '↑' : '↓'}
                  </span>
                )}
              </th>
            </tr>
          </thead>
          <tbody>
            {sorted.map((row) => (
              <tr
                key={row.address}
                className="border-t border-gray-200 dark:border-gray-700"
              >
                <td className="px-2 py-1">{addressLink(row.address)}</td>
                <td className="px-2 py-1">{row.blocks.toLocaleString()}</td>
                <td className="px-2 py-1">
                  {row.revenue != null ? `$${formatUsd(row.revenue)}` : 'N/A'}
                </td>
                <td className="px-2 py-1">{`$${formatUsd(row.cost)}`}</td>
                <td className="px-2 py-1">
                  {row.profit != null ? `$${formatUsd(row.profit)}` : 'N/A'}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
};
