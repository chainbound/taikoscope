import React from 'react';
import { MetricData, TimeRange } from '../types';

interface ProfitCalculatorProps {
  metrics: MetricData[];
  timeRange: TimeRange;
  cloudCost: number;
  proverCost: number;
  onCloudCostChange: (v: number) => void;
  onProverCostChange: (v: number) => void;
}

export const ProfitCalculator: React.FC<ProfitCalculatorProps> = ({
  cloudCost,
  proverCost,
  onCloudCostChange,
  onProverCostChange,
}) => {
  return (
    <div className="mt-6 p-4 border border-gray-200 dark:border-gray-700 rounded-md bg-gray-50 dark:bg-gray-800">
      <h2 className="text-lg font-semibold mb-2">Hardware Costs</h2>
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
    </div>
  );
};
