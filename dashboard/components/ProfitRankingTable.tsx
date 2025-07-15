import React from 'react';
import useSWR from 'swr';
import { TimeRange } from '../types';
import {
  fetchSequencerDistribution,
  fetchL2FeesComponents,
} from '../services/apiService';
import * as apiService from '../services/apiService';
import { getSequencerAddress } from '../sequencerConfig';
import { addressLink, formatEth, formatDecimal } from '../utils';
import { calculateProfit } from '../utils/profit';
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
    minimumFractionDigits: 3,
    maximumFractionDigits: 3,
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


  const { data: feeRes } = useSWR(['profitRankingData', timeRange], () =>
    fetchL2FeesComponents(timeRange),
  );
  
  const feeDataMap = React.useMemo(() => {
    const map = new Map<string, apiService.SequencerFee>();
    feeRes?.data?.sequencers.forEach((f) => {
      map.set(f.address.toLowerCase(), f);
    });
    return map;
  }, [feeRes]);

  const batchCounts = React.useMemo(() => {
    const map = new Map<string, number>();
    if (feeRes?.data?.batches) {
      feeRes.data.batches.forEach((batch) => {
        const current = map.get(batch.sequencer) || 0;
        map.set(batch.sequencer, current + 1);
      });
    }
    return map;
  }, [feeRes]);

  const [sortBy, setSortBy] = React.useState<
    'name' | 'blocks' | 'batches' | 'revenue' | 'cost' | 'profit' | 'ratio'
  >('ratio');
  const [sortDirection, setSortDirection] = React.useState<'asc' | 'desc'>(
    'desc',
  );

  const hours = rangeToHours(timeRange);
  const MONTH_HOURS = 30 * 24;
  const costPerSeqUsd = ((cloudCost + proverCost) / MONTH_HOURS) * hours;
  const costPerSeqEth = ethPrice ? costPerSeqUsd / ethPrice : 0;

  const rows = sequencers.map((seq) => {
    const addr = seq.address || getSequencerAddress(seq.name) || '';
    const batchCount = batchCounts.get(addr.toLowerCase()) ?? null;
    const fees = feeDataMap.get(addr.toLowerCase());
    const proveEth = (fees?.prove_cost ?? 0) / 1e9;
    const verifyEth = 0;
    const extraEth = proveEth + verifyEth;
    const extraUsd = extraEth * ethPrice;
    if (!fees) {
      return {
        name: seq.name,
        address: addr,
        blocks: seq.value,
        batches: batchCount,
        revenueEth: null as number | null,
        revenueUsd: null as number | null,
        costEth: costPerSeqEth + extraEth,
        costUsd: costPerSeqUsd + extraUsd,
        profitEth: null as number | null,
        profitUsd: null as number | null,
        ratio: null as number | null,
      };
    }
    const revenueEth =
      ((fees.priority_fee ?? 0) + (fees.base_fee ?? 0) * 0.75) / 1e9;
    const l1CostEth = (fees.l1_data_cost ?? 0) / 1e9;
    const revenueUsd = revenueEth * ethPrice;
    const l1CostUsd = l1CostEth * ethPrice;
    const costEth = costPerSeqEth + l1CostEth + extraEth;
    const costUsd = costPerSeqUsd + l1CostUsd + extraUsd;
    const { profitEth, profitUsd } = calculateProfit({
      priorityFee: fees.priority_fee,
      baseFee: fees.base_fee,
      l1DataCost: fees.l1_data_cost,
      proveCost: fees.prove_cost,

      hardwareCostUsd: costPerSeqUsd,
      ethPrice,
    });
    const ratio = costEth > 0 ? revenueEth / costEth : null;
    return {
      name: seq.name,
      address: addr,
      blocks: seq.value,
      batches: batchCount,
      revenueEth,
      revenueUsd,
      costEth,
      costUsd,
      profitEth,
      profitUsd,
      ratio,
    };
  });

  const sorted = React.useMemo(() => {
    const data = [...rows];
    data.sort((a, b) => {
      const key =
        sortBy === 'revenue'
          ? 'revenueEth'
          : sortBy === 'cost'
            ? 'costEth'
            : sortBy === 'profit'
              ? 'profitEth'
              : sortBy === 'ratio'
                ? 'ratio'
                : sortBy;
      const aVal = (a as any)[key];
      const bVal = (b as any)[key];
      let cmp = 0;
      // Handle numeric columns (blocks, revenue, cost, profit and ratio) including null values
      if (
        sortBy === 'blocks' ||
        sortBy === 'batches' ||
        sortBy === 'revenue' ||
        sortBy === 'cost' ||
        sortBy === 'profit' ||
        sortBy === 'ratio'
      ) {
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

  if (!feeRes) {
    return (
      <div className="flex items-center justify-center h-20 text-gray-500 dark:text-gray-400">
        Loading...
      </div>
    );
  }

  const handleSort = (
    column:
      | 'name'
      | 'blocks'
      | 'batches'
      | 'revenue'
      | 'cost'
      | 'profit'
      | 'ratio',
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
                onClick={() => handleSort('batches')}
              >
                Batches
                {sortBy === 'batches' && (
                  <span className="ml-1">
                    {sortDirection === 'asc' ? '↑' : '↓'}
                  </span>
                )}
              </th>
              <th
                className="px-2 py-1 text-left cursor-pointer select-none"
                onClick={() => handleSort('revenue')}
              >
                Revenue
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
                Cost
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
                Profit
                {sortBy === 'profit' && (
                  <span className="ml-1">
                    {sortDirection === 'asc' ? '↑' : '↓'}
                  </span>
                )}
              </th>
              <th
                className="px-2 py-1 text-left cursor-pointer select-none"
                onClick={() => handleSort('ratio')}
              >
                Revenue-to-Cost Ratio
                {sortBy === 'ratio' && (
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
                <td className="px-2 py-1">
                  {addressLink(row.address, row.name)}
                </td>
                <td className="px-2 py-1">{row.blocks.toLocaleString()}</td>
                <td className="px-2 py-1">
                  {row.batches != null ? row.batches.toLocaleString() : 'N/A'}
                </td>
                <td
                  className="px-2 py-1"
                  title={
                    row.revenueUsd != null
                      ? `$${formatUsd(row.revenueUsd)}`
                      : undefined
                  }
                >
                  {row.revenueEth != null
                    ? formatEth(row.revenueEth * 1e9, 4)
                    : 'N/A'}
                </td>
                <td className="px-2 py-1" title={`$${formatUsd(row.costUsd)}`}>
                  {formatEth(row.costEth * 1e9, 4)}
                </td>
                <td
                  className="px-2 py-1"
                  title={
                    row.profitUsd != null
                      ? `$${formatUsd(row.profitUsd)}`
                      : undefined
                  }
                >
                  {row.profitEth != null
                    ? formatEth(row.profitEth * 1e9, 4)
                    : 'N/A'}
                </td>
                <td className="px-2 py-1">
                  {row.ratio != null ? formatDecimal(row.ratio) : 'N/A'}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
};
