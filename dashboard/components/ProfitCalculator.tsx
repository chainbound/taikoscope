import React, { useEffect, useState } from 'react';
import { MetricData } from '../types';
import { findMetricValue } from '../utils';
import { getEthPrice } from '../services/priceService';

interface ProfitCalculatorProps {
  metrics: MetricData[];
}

export const ProfitCalculator: React.FC<ProfitCalculatorProps> = ({
  metrics,
}) => {
  const feeStr = findMetricValue(metrics, 'transaction fee');
  const fee = parseFloat(feeStr.replace(/[^0-9.]/g, '')) || 0;

  const [cloudCost, setCloudCost] = useState(0);
  const [proverCost, setProverCost] = useState(0);
  const [ethPrice, setEthPrice] = useState(0);
  const [ethPriceError, setEthPriceError] = useState(false);

  useEffect(() => {
    getEthPrice()
      .then((p) => {
        setEthPrice(p);
        setEthPriceError(false);
      })
      .catch((err) => {
        console.error('Failed to fetch ETH price', err);
        setEthPriceError(true);
      });
  }, []);

  const profit = fee * ethPrice - cloudCost - proverCost;

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
          <span className="text-red-500 ml-2 text-xs">(ETH price unavailable)</span>
        )}
      </p>
    </div>
  );
};
