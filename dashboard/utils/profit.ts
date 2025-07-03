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

const WEI_TO_ETH = 1e9;

export const calculateProfit = ({
  priorityFee = 0,
  baseFee = 0,
  l1DataCost = 0,
  proveCost = 0,
  hardwareCostUsd,
  ethPrice,
}: ProfitParams): ProfitResult => {
  const revenueEth = ((priorityFee ?? 0) + (baseFee ?? 0) * 0.75) / WEI_TO_ETH;
  const revenueUsd = revenueEth * ethPrice;
  const costEth =
    hardwareCostUsd / ethPrice +
    ((l1DataCost ?? 0) + (proveCost ?? 0)) / WEI_TO_ETH;
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
