import { describe, it, expect } from 'vitest';
import { calculateProfit } from '../utils/profit';

describe('calculateProfit', () => {
  it('computes positive profit', () => {
    const res = calculateProfit({
      priorityFee: 2,
      baseFee: 1,
      l1DataCost: 0.5,
      proveCost: 0.1,

      hardwareCostUsd: 100,
      ethPrice: 10,
    });
    expect(res.profitEth).toBeCloseTo(
      (2 + 0.75) - (100 / 10 + (0.5 + 0.1))
    );
    expect(res.profitUsd).toBeCloseTo(res.profitEth * 10);
  });

  it('handles negative profit', () => {
    const res = calculateProfit({
      priorityFee: 0,
      baseFee: 0,
      l1DataCost: 1,
      proveCost: 0,

      hardwareCostUsd: 50,
      ethPrice: 5,
    });
    expect(res.profitEth).toBeCloseTo(-((50 / 5) + 1));
    expect(res.profitUsd).toBeCloseTo(res.profitEth * 5);
  });
});
