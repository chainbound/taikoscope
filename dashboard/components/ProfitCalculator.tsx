import React, { useState } from 'react';
import { MetricData, TimeRange } from '../types';
import { findMetricValue } from '../utils';
import { useEthPrice } from '../services/priceService';

interface ProfitCalculatorProps {
  metrics: MetricData[];
  timeRange: TimeRange;
}

export const ProfitCalculator: React.FC<ProfitCalculatorProps> = ({
  metrics,
  timeRange,
}) => {
  const priorityStr = findMetricValue(metrics, 'priority fee');
  const baseStr = findMetricValue(metrics, 'base fee');
  const priority = parseFloat(priorityStr.replace(/[^0-9.]/g, '')) || 0;
  const base = parseFloat(baseStr.replace(/[^0-9.]/g, '')) || 0;
  const totalFee = priority + base;

  const [cloudCost, setCloudCost] = useState(100);
  const [proverCost, setProverCost] = useState(100);
  const { data: ethPrice = 0, error: ethPriceError } = useEthPrice();

  const HOURS_IN_MONTH = 30 * 24;
  const RANGE_HOURS: Record<TimeRange, number> = {
    '15m': 0.25,
    '1h': 1,
    '24h': 24,
  };
  const hours = RANGE_HOURS[timeRange];

  const scaledCloudCost = (cloudCost / HOURS_IN_MONTH) * hours;
  const scaledProverCost = (proverCost / HOURS_IN_MONTH) * hours;
  const profit = totalFee * ethPrice - scaledCloudCost - scaledProverCost;

  return (
    <div className="mt-6 p-4 border border-gray-200 dark:border-gray-700 rounded-md bg-gray-50 dark:bg-gray-800">
      <h2 className="text-lg font-semibold mb-2">Profit Calculator</h2>
      <div className="flex flex-col sm:flex-row sm:space-x-4 space-y-2 sm:space-y-0">
        <label className="flex flex-col text-sm">
          Monthly Cloud Cost ($)
          <input
            type="number"
            className="p-1 border rounded-md"
            value={cloudCost}
            onChange={(e) => setCloudCost(Number(e.target.value))}
          />
        </label>
        <label className="flex flex-col text-sm">
          Prover Cost ($)
          <input
            type="number"
            className="p-1 border rounded-md"
            value={proverCost}
            onChange={(e) => setProverCost(Number(e.target.value))}
          />
        </label>
      </div>
      <p className="mt-3 text-sm">
        Profit: <span className="font-semibold">${profit.toFixed(2)}</span>
        {ethPriceError && (
          <span className="text-red-500 ml-2 text-xs">
            (ETH price unavailable)
          </span>
        )}
      </p>
    </div>
  );
};
