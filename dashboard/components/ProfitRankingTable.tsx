import React from 'react';
import useSWR from 'swr';
import { TimeRange } from '../types';
import {
  fetchSequencerDistribution,
  fetchBatchFeeComponents,
} from '../services/apiService';
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


  const { data: batchRes } = useSWR(['profitRankingBatchesAll', timeRange], () =>
    fetchBatchFeeComponents(timeRange),
  );
  const feeDataMap = React.useMemo(() => {
    const map = new Map<string, {
      priority: number;
      base: number;
      l1: number;
      prove: number;
      verify: number;
      count: number;
    }>();
    if (Array.isArray(batchRes?.data)) {
      batchRes.data.forEach((b) => {
        const addr = (b.sequencer || '').toLowerCase();
        if (!addr) return;
      const entry = map.get(addr) || {
        priority: 0,
        base: 0,
        l1: 0,
        prove: 0,
        verify: 0,
        count: 0,
      };
      entry.priority += b.priority;
      entry.base += b.base;
      entry.l1 += b.l1Cost ?? 0;
      entry.prove += b.amortizedProveCost ?? 0;
      entry.verify += b.amortizedVerifyCost ?? 0;
      entry.count += 1;
      map.set(addr, entry);
      });
    }
    return map;
  }, [batchRes]);



  const [sortBy, setSortBy] = React.useState<
    'name' | 'blocks' | 'batches' | 'revenue' | 'cost' | 'profit' | 'ratio'
  >('ratio');
  const [sortDirection, setSortDirection] = React.useState<'asc' | 'desc'>(
    'desc',
  );

  const hours = rangeToHours(timeRange);
  const MONTH_HOURS = 30 * 24;
  const totalBatchCount = React.useMemo(() => {
    let count = 0;
    feeDataMap.forEach((v) => {
      count += v.count;
    });
    return count;
  }, [feeDataMap]);
  const hardwarePerBatchUsd =
    totalBatchCount > 0
      ? ((cloudCost + proverCost) / MONTH_HOURS) * (hours / totalBatchCount)
      : 0;

  const rows = sequencers.map((seq) => {
    const addr = seq.address || getSequencerAddress(seq.name) || '';
    const fees = feeDataMap.get(addr.toLowerCase());
    const batchCount = fees?.count ?? 0;
    const costPerSeqUsd = hardwarePerBatchUsd * batchCount;
    const costPerSeqEth = ethPrice ? costPerSeqUsd / ethPrice : 0;
    const proveEth = (fees?.prove ?? 0) / 1e18;
    const verifyEth = (fees?.verify ?? 0) / 1e18;
    const extraEth = proveEth + verifyEth;
    const extraUsd = extraEth * ethPrice;
    if (!fees) {
      return {
        name: seq.name,
        address: addr,
        blocks: seq.value,
        batches: batchCount || null,
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
      ((fees.priority ?? 0) + (fees.base ?? 0) * 0.75) / 1e18;
    const l1CostEth = (fees.l1 ?? 0) / 1e18;
    const revenueUsd = revenueEth * ethPrice;
    const l1CostUsd = l1CostEth * ethPrice;
    const costEth = costPerSeqEth + l1CostEth + extraEth;
    const costUsd = costPerSeqUsd + l1CostUsd + extraUsd;
    const { profitEth, profitUsd } = calculateProfit({
      priorityFee: fees.priority,
      baseFee: fees.base,
      l1DataCost: fees.l1,
      proveCost: fees.prove,
      verifyCost: fees.verify,
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

  if (!batchRes) {
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
                Income/Cost
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
                    ? formatEth(row.revenueEth * 1e18)
                    : 'N/A'}
                </td>
                <td className="px-2 py-1" title={`$${formatUsd(row.costUsd)}`}>
                  {formatEth(row.costEth * 1e18)}
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
                    ? formatEth(row.profitEth * 1e18)
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
