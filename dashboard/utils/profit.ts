export interface ProfitParams {
  priorityFee?: number | null;
  baseFee?: number | null;
  l1DataCost?: number | null;
  proveCost?: number | null;
  verifyCost?: number | null;
  hardwareCostUsd: number;
  ethPrice: number;
}

export interface ProfitResult {
  profitEth: number;
  profitUsd: number;
}

const WEI_TO_ETH = 1e18;

export const calculateProfit = ({
  priorityFee = 0,
  baseFee = 0,
  l1DataCost = 0,
  proveCost = 0,
  verifyCost = 0,
  hardwareCostUsd,
  ethPrice,
}: ProfitParams): ProfitResult => {
  const revenueEth = ((priorityFee ?? 0) + (baseFee ?? 0) * 0.75) / WEI_TO_ETH;
  const revenueUsd = revenueEth * ethPrice;
  const costEth =
    hardwareCostUsd / ethPrice +
    ((l1DataCost ?? 0) + (proveCost ?? 0) + (verifyCost ?? 0)) / WEI_TO_ETH;
  const costUsd = costEth * ethPrice;
  const profitUsd = revenueUsd - costUsd;
  const profitEth = revenueEth - costEth;
  return { profitEth, profitUsd };
};
