import { describe, it, expect } from 'vitest';
import { calculateProfit, calculateNetProfit, SEQUENCER_BASE_FEE_RATIO } from '../utils/profit';

describe('calculateProfit', () => {
  it('computes positive profit', () => {
    const res = calculateProfit({
      priorityFee: 2e9,
      baseFee: 1e9,
      l1DataCost: 5e8,
      proveCost: 1e8,

      hardwareCostUsd: 100,
      ethPrice: 10,
    });
    expect(res.revenueEth).toBeCloseTo(2.75);
    expect(res.costEth).toBeCloseTo(10.6);
    expect(res.profitEth).toBeCloseTo(2.75 - 10.6);
    expect(res.revenueUsd).toBeCloseTo(27.5);
    expect(res.costUsd).toBeCloseTo(106);
    expect(res.profitUsd).toBeCloseTo(res.profitEth * 10);
  });

  it('handles negative profit', () => {
    const res = calculateProfit({
      priorityFee: 0,
      baseFee: 0,
      l1DataCost: 1e9,
      proveCost: 0,

      hardwareCostUsd: 50,
      ethPrice: 5,
    });
    expect(res.revenueEth).toBeCloseTo(0);
    expect(res.costEth).toBeCloseTo(11);
    expect(res.profitEth).toBeCloseTo(-11);
    expect(res.revenueUsd).toBeCloseTo(0);
    expect(res.costUsd).toBeCloseTo(55);
    expect(res.profitUsd).toBeCloseTo(-55);
  });

  it('calculates net profit in gwei', () => {
    const profit = calculateNetProfit({
      priorityFee: 10,
      baseFee: 20,
      l1DataCost: 5,
      proveCost: 5,
    });
    expect(profit).toBeCloseTo(10 + 20 * SEQUENCER_BASE_FEE_RATIO - 5 - 5);
  });

  it('handles zero ETH price without NaN', () => {
    const res = calculateProfit({
      priorityFee: 2e9,
      baseFee: 1e9,
      l1DataCost: 5e8,
      proveCost: 1e8,
      hardwareCostUsd: 100,
      ethPrice: 0,
    });
    expect(res.revenueEth).toBeCloseTo(2.75);
    expect(res.costEth).toBeCloseTo(0.6); // Only l1DataCost + proveCost, no hardware cost
    expect(res.profitEth).toBeCloseTo(2.15);
    expect(res.revenueUsd).toBe(0);
    expect(res.costUsd).toBe(0);
    expect(res.profitUsd).toBe(0);
    expect(Number.isFinite(res.costEth)).toBe(true);
    expect(Number.isFinite(res.profitEth)).toBe(true);
  });
});
