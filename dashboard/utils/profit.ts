export interface ProfitParams {
  priorityFee?: number | null;
  baseFee?: number | null;
  l1DataCost?: number | null;
  proveCost?: number | null;

  hardwareCostUsd: number;
  ethPrice: number;
}

export interface ProfitResult {
  revenueEth: number;
  revenueUsd: number;
  costEth: number;
  costUsd: number;
  profitEth: number;
  profitUsd: number;
}

const GWEI_TO_ETH = 1e9;

export const calculateProfit = ({
  priorityFee = 0,
  baseFee = 0,
  l1DataCost = 0,
  proveCost = 0,
  hardwareCostUsd,
  ethPrice,
}: ProfitParams): ProfitResult => {
  const revenueEth = ((priorityFee ?? 0) + (baseFee ?? 0) * 0.75) / GWEI_TO_ETH;
  const revenueUsd = revenueEth * ethPrice;
  const hardwareCostEth = ethPrice && ethPrice > 0 ? hardwareCostUsd / ethPrice : 0;
  const costEth =
    hardwareCostEth +
    ((l1DataCost ?? 0) + (proveCost ?? 0)) / GWEI_TO_ETH;
  const costUsd = costEth * ethPrice;
  const profitUsd = revenueUsd - costUsd;
  const profitEth = revenueEth - costEth;
  return {
    revenueEth,
    revenueUsd,
    costEth,
    costUsd,
    profitEth,
    profitUsd,
  };
};

export interface NetProfitParams {
  priorityFee?: number | null;
  baseFee?: number | null;
  l1DataCost?: number | null;
  proveCost?: number | null;
}

/**
 * Calculate net profit in gwei without hardware costs.
 */
export const calculateNetProfit = ({
  priorityFee = 0,
  baseFee = 0,
  l1DataCost = 0,
  proveCost = 0,
}: NetProfitParams): number => {
  return (
    (priorityFee ?? 0) +
    (baseFee ?? 0) * 0.75 -
    (l1DataCost ?? 0) -
    (proveCost ?? 0)
  );
};
