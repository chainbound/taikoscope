import React from 'react';
import useSWR from 'swr';
import { TimeRange } from '../types';
import {
  fetchSequencerDistribution,
  fetchL2Fees,
} from '../services/apiService';
import * as apiService from '../services/apiService';
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

  const { data: feeRes } = useSWR(['profitRankingFees', timeRange], () =>
    fetchL2Fees(timeRange),
  );
  const feeDataMap = React.useMemo(() => {
    const map = new Map<string, apiService.SequencerFee>();
    feeRes?.data?.sequencers.forEach((f) => {
      map.set(f.address.toLowerCase(), f);
    });
    return map;
  }, [feeRes]);

  if (!feeRes) {
    return (
      <div className="flex items-center justify-center h-20 text-gray-500 dark:text-gray-400">
        Loading...
      </div>
    );
  }

  const hours = rangeToHours(timeRange);
  const MONTH_HOURS = 30 * 24;
  const costPerSeq = ((cloudCost + proverCost) / MONTH_HOURS) * hours;

  const rows = sequencers.map((seq) => {
    const addr = seq.address || getSequencerAddress(seq.name) || '';
    const fees = feeDataMap.get(addr.toLowerCase());
    if (!fees) {
      return {
        name: seq.name,
        blocks: seq.value,
        profit: null as number | null,
      };
    }
    const revenueEth =
      ((fees.priority_fee ?? 0) +
        (fees.base_fee ?? 0) * 0.75 -
        (fees.l1_data_cost ?? 0)) /
      1e18;
    const profit = revenueEth * ethPrice - costPerSeq;
    return { name: seq.name, blocks: seq.value, profit };
  });

  const [sortBy, setSortBy] = React.useState<'name' | 'blocks' | 'profit'>(
    'profit',
  );
  const [sortDirection, setSortDirection] = React.useState<'asc' | 'desc'>(
    'desc',
  );

  const sorted = React.useMemo(() => {
    const data = [...rows];
    data.sort((a, b) => {
      const aVal = a[sortBy];
      const bVal = b[sortBy];
      let cmp = 0;
      // Handle numeric columns (blocks and profit) including null values
      if (sortBy === 'blocks' || sortBy === 'profit') {
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

  const handleSort = (column: 'name' | 'blocks' | 'profit') => {
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
