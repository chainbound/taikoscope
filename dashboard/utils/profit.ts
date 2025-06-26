export interface ProfitParams {
  priorityWei?: number;
  baseWei?: number;
  l1CostWei?: number;
  proveCostWei?: number;
  verifyCostWei?: number;
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

const WEI_TO_ETH = 1e18;

export const calculateProfit = ({
  priorityWei = 0,
  baseWei = 0,
  l1CostWei = 0,
  proveCostWei = 0,
  verifyCostWei = 0,
  hardwareCostUsd,
  ethPrice,
}: ProfitParams): ProfitResult => {
  const revenueEth = (priorityWei + baseWei * 0.75) / WEI_TO_ETH;
  const revenueUsd = revenueEth * ethPrice;

  const l1CostEth = l1CostWei / WEI_TO_ETH;
  const proveEth = proveCostWei / WEI_TO_ETH;
  const verifyEth = verifyCostWei / WEI_TO_ETH;
  const hardwareEth = ethPrice > 0 ? hardwareCostUsd / ethPrice : 0;

  const costEth = hardwareEth + l1CostEth + proveEth + verifyEth;
  const costUsd = costEth * ethPrice;

  const profitEth = revenueEth - costEth;
  const profitUsd = profitEth * ethPrice;

  return {
    revenueEth,
    revenueUsd,
    costEth,
    costUsd,
    profitEth,
    profitUsd,
  };
};
