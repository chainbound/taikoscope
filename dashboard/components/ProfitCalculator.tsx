import React from 'react';
import { TimeRange } from '../types';

interface ProfitCalculatorProps {
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
    <div className="mt-6 p-4 border border-border rounded-md bg-card text-card-fg">
      <h2 className="text-lg font-semibold mb-2">Hardware Costs</h2>
      <div className="flex flex-col sm:flex-row sm:space-x-4 space-y-2 sm:space-y-0">
        <label className="flex flex-col text-sm">
          Monthly Cloud Cost ($)
          <input
            type="number"
            min={0}
            className="p-1 border border-border rounded-md bg-card text-card-fg"
            value={cloudCost}
            onFocus={(e) => e.target.select()}
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
            className="p-1 border border-border rounded-md bg-card text-card-fg"
            value={proverCost}
            onFocus={(e) => e.target.select()}
            onChange={(e) =>
              onProverCostChange(Math.max(0, Number(e.target.value)))
            }
          />
        </label>
      </div>
    </div>
  );
};
