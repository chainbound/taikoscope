import React from 'react';
import type { L2FeesComponentsResponse } from '../services/apiService';
import { useEthPrice } from '../services/priceService';
import { TimeRange } from '../types';
import { rangeToHours } from '../utils/timeRange';
import { formatEth, l1TxLink, addressLink, formatDecimal } from '../utils';
import { getSequencerName } from '../sequencerConfig';
import { calculateProfit } from '../utils/profit';

interface BlockProfitTablesProps {
  timeRange: TimeRange;
  cloudCost: number;
  proverCost: number;
  address?: string;
  /** Total number of sequencers for scaling network-wide costs */
  totalSequencers?: number;
  /** Pre-fetched L2 fees + components data to avoid duplicate requests */
  feesData?: L2FeesComponentsResponse | null;
}

const formatUsd = (value: number): string => {
  const abs = Math.abs(value);
  if (abs >= 1000) return Math.trunc(value).toLocaleString();
  return value.toLocaleString(undefined, {
    minimumFractionDigits: 3,
    maximumFractionDigits: 3,
  });
};

export const BlockProfitTables: React.FC<BlockProfitTablesProps> = ({
  timeRange,
  cloudCost,
  proverCost,
  address,
  totalSequencers,
  feesData,
}) => {
  const { data: ethPrice = 0 } = useEthPrice();
  const batchData =
    feesData?.batches
      ?.filter((b) => !address || b.sequencer === address)
      .map((b) => ({
        batch: b.batch_id,
        txHash: b.l1_tx_hash,
        sequencer: b.sequencer,
        priority: b.priority_fee,
        base: b.base_fee,
        l1Cost: b.l1_data_cost,
        proveCost: b.prove_cost,
      })) ?? [];
  const batchCount = batchData.length;
  const HOURS_IN_MONTH = 30 * 24;
  const hours = rangeToHours(timeRange);

  const seqCount = address ? 1 : Math.max(1, totalSequencers ?? 1);
  const operationalCostPerBatchUsd = batchCount > 0
    ? ((cloudCost + proverCost) * seqCount) / HOURS_IN_MONTH * (hours / batchCount)
    : 0;

  const profits = batchData.map((b) => {
    const { revenueEth, costEth, profitEth } = calculateProfit({
      priorityFee: b.priority,
      baseFee: b.base,
      l1DataCost: b.l1Cost ?? 0,
      proveCost: b.proveCost ?? 0,

      hardwareCostUsd: operationalCostPerBatchUsd,
      ethPrice,
    });
    const profitGwei = profitEth * 1e9;
    const revenueGwei = revenueEth * 1e9;
    const costGwei = costEth * 1e9;

    return {
      batch: b.batch,
      txHash: b.txHash,
      sequencer: b.sequencer,
      revenue: revenueGwei,
      cost: costGwei,
      profit: profitGwei,
      profitEth, // Store ETH value for sorting and display
      revenueEth,
      costEth,
      ratio: costEth > 0 ? revenueEth / costEth : null,
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
      | {
        batch: number;
        txHash: string;
        sequencer: string;
        revenue: number;
        cost: number;
        profit: number;
        profitEth: number;
        revenueEth: number;
        costEth: number;
        ratio: number | null;
      }[]
      | null,
  ) => (
    <div>
      <h3 className="text-lg font-semibold mb-2">{title}</h3>
      <div className="overflow-x-auto">
        <table className="min-w-full table-fixed border border-gray-100 dark:border-[#475569] divide-y divide-gray-100 dark:divide-[#475569] bg-card dark:bg-[rgba(30,41,59,0.85)] text-card-fg">
          <colgroup>
            {/* Batch */}
            <col className="w-[12%]" />
            {/* Sequencer - widened to increase gap before Revenue */}
            <col className="w-[16%]" />
            {/* Numeric columns */}
            <col className="w-[18%]" />
            <col className="w-[18%]" />
            <col className="w-[18%]" />
            <col className="w-[18%]" />
          </colgroup>
          <thead>
            <tr>
              <th className="px-2 py-1 text-left">Batch</th>
              <th className="px-2 py-1 text-left">Sequencer</th>
              <th className="px-2 py-1 text-left tabular-nums">Revenue</th>
              <th className="px-2 py-1 text-left tabular-nums">Cost</th>
              <th className="px-2 py-1 text-left tabular-nums">Profit</th>
              <th className="px-2 py-1 text-left tabular-nums">Revenue-to-Cost Ratio</th>
            </tr>
          </thead>
          <tbody>
            {items?.map((b) => (
              <tr
                key={b.batch}
                className="border-t border-gray-100 dark:border-[#475569]"
              >
                <td className="px-2 py-1 whitespace-nowrap">
                  {l1TxLink(b.txHash, b.batch.toLocaleString())}
                </td>
                <td className="px-2 py-1 whitespace-nowrap">{addressLink(b.sequencer, getSequencerName(b.sequencer))}</td>
                <td
                  className="px-2 py-1 text-left tabular-nums"
                  title={`$${formatUsd(b.revenueEth * ethPrice)}`}
                >
                  {formatEth(b.revenue, 4)}
                </td>
                <td
                  className="px-2 py-1 text-left tabular-nums"
                  title={`$${formatUsd(b.costEth * ethPrice)}`}
                >
                  {formatEth(b.cost, 4)}
                </td>
                <td
                  className="px-2 py-1 text-left tabular-nums"
                  title={`$${formatUsd(b.profitEth * ethPrice)}`}
                >
                  {formatEth(b.profit, 4)}
                </td>
                <td className="px-2 py-1 text-left tabular-nums">
                  {b.ratio != null ? formatDecimal(b.ratio) : 'N/A'}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );

  return (
    <div className="mt-6 grid grid-cols-1 gap-4 md:gap-6">
      {renderTable('Top 5 Profitable Batches', topBatches)}
      {renderTable('Least 5 Profitable Batches', bottomBatches)}
    </div>
  );
};
