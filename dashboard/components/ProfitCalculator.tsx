import React from 'react';
import { MetricData, TimeRange } from '../types';
import { findMetricValue } from '../utils';
import { rangeToHours } from '../utils/timeRange';
import { useEthPrice } from '../services/priceService';

interface ProfitCalculatorProps {
  metrics: MetricData[];
  timeRange: TimeRange;
  cloudCost: number;
  proverCost: number;
  onCloudCostChange: (v: number) => void;
  onProverCostChange: (v: number) => void;
}

const formatTimeRangeLabel = (range: TimeRange): string => {
  const match = range.trim().match(/^(\d+)([mh])$/i);
  if (!match) return range;
  const value = parseInt(match[1], 10);
  const unit = match[2].toLowerCase() === 'h' ? 'hour' : 'minute';
  const plural = value === 1 ? '' : 's';
  return `last ${value} ${unit}${plural}`;
};

export const ProfitCalculator: React.FC<ProfitCalculatorProps> = ({
  metrics,
  timeRange,
  cloudCost,
  proverCost,
  onCloudCostChange,
  onProverCostChange,
}) => {
  const priorityStr = findMetricValue(metrics, 'priority fee');
  const baseStr = findMetricValue(metrics, 'base fee');
  const l1DataCostStr = findMetricValue(metrics, 'l1 data cost');
  const priority = parseFloat(priorityStr.replace(/[^0-9.]/g, '')) || 0;
  const base = parseFloat(baseStr.replace(/[^0-9.]/g, '')) || 0;
  const l1DataCost = parseFloat(l1DataCostStr.replace(/[^0-9.]/g, '')) || 0;
  const totalFee = priority + base - l1DataCost;

  const { data: ethPrice = 0, error: ethPriceError } = useEthPrice();

  const HOURS_IN_MONTH = 30 * 24;
  const hours = rangeToHours(timeRange);

  const scaledCloudCost = (cloudCost / HOURS_IN_MONTH) * hours;
  const scaledProverCost = (proverCost / HOURS_IN_MONTH) * hours;
  const profit = totalFee * ethPrice - scaledCloudCost - scaledProverCost;

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

  return (
    <div className="mt-6 p-4 border border-gray-200 dark:border-gray-700 rounded-md bg-gray-50 dark:bg-gray-800">
      <h2 className="text-lg font-semibold mb-2">Profit Calculator</h2>
      <div className="flex flex-col sm:flex-row sm:space-x-4 space-y-2 sm:space-y-0">
        <label className="flex flex-col text-sm">
          Monthly Cloud Cost ($)
          <input
            type="number"
            min={0}
            className="p-1 border rounded-md"
            value={cloudCost}
            onChange={(e) =>
              onCloudCostChange(Math.max(0, Number(e.target.value)))
            }
          />
        </label>
        <label className="flex flex-col text-sm">
          Prover Cost ($)
          <input
            type="number"
            min={0}
            className="p-1 border rounded-md"
            value={proverCost}
            onChange={(e) =>
              onProverCostChange(Math.max(0, Number(e.target.value)))
            }
          />
        </label>
      </div>
      <p className="mt-3 text-sm">
        Profit ({formatTimeRangeLabel(timeRange)}):{' '}
        <span className="font-semibold">${formatProfit(profit)}</span>
        {ethPriceError && (
          <span className="text-red-500 ml-2 text-xs">
            (ETH price unavailable)
          </span>
        )}
      </p>
    </div>
  );
};
