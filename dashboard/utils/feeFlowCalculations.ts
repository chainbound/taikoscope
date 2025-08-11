import { SEQUENCER_BASE_FEE_RATIO, calculateProfit } from './profit';
import { getSequencerName } from '../sequencerConfig';

const GWEI_TO_ETH = 1e9;

export interface SequencerFeeData {
  address: string;
  priority_fee?: number | null;
  base_fee?: number | null;
  l1_data_cost?: number | null;
  prove_cost?: number | null;
}

export interface ProcessedSequencerData {
  address: string;
  shortAddress: string;
  priorityUsd: number;
  baseUsd: number;
  revenue: number;
  revenueGwei: number;
  profit: number;
  profitGwei: number;
  actualHardwareCost: number;
  actualL1Cost: number;
  actualProveCost: number;
  l1CostUsd: number;
  subsidyUsd: number;
  subsidyGwei: number;
  actualHardwareCostGwei: number;
  actualL1CostGwei: number;
  actualProveCostGwei: number;
}

export interface FeeConversionParams {
  priorityFee?: number | null;
  baseFee?: number | null;
  l1DataCost?: number | null;
  proveCost?: number | null;
  ethPrice: number;
}

export interface FeeConversionResult {
  priorityFeeUsd: number;
  baseFeeUsd: number;
  l1DataCostTotalUsd: number;
  l1ProveCost: number;
  baseFeeDaoUsd: number;
}

export interface SankeyNode {
  name: string;
  value: number;
  gwei?: number;
  usd?: boolean;
  depth: number;
  address?: string;
  addressLabel?: string;
  profitNode?: boolean;
  revenueNode?: boolean;
  subsidyNode?: boolean;
  hideLabel?: boolean;
}

export interface SankeyLink {
  source: number;
  target: number;
  value: number;
}

export interface SankeyChartData {
  nodes: SankeyNode[];
  links: SankeyLink[];
}


export interface FallbackCalculationParams {
  priorityFeeUsd: number;
  baseFeeUsd: number;
  baseFeeDaoUsd: number;
  l1DataCostTotalUsd: number;
  l1ProveCost: number;
  totalHardwareCost: number;
  priorityFee?: number | null;
  baseFee?: number | null;
  ethPrice: number;
}

// Helper function to ensure finite values
export const safeValue = (value: number): number =>
  (isFinite(value) ? value : 0);

/**
 * Convert fees from Gwei to USD
 */
export const convertFeesToUsd = ({
  priorityFee = 0,
  baseFee = 0,
  l1DataCost = 0,
  proveCost = 0,
  ethPrice,
}: FeeConversionParams): FeeConversionResult => {
  const priorityFeeUsd = safeValue(
    ((priorityFee ?? 0) / GWEI_TO_ETH) * ethPrice,
  );
  const baseFeeUsd = safeValue(((baseFee ?? 0) / GWEI_TO_ETH) * ethPrice);
  const l1DataCostTotalUsd = safeValue(
    ((l1DataCost ?? 0) / GWEI_TO_ETH) * ethPrice,
  );
  const l1ProveCost = safeValue(
    ((proveCost ?? 0) / GWEI_TO_ETH) * ethPrice,
  );
  const baseFeeDaoUsd = safeValue(baseFeeUsd * 0.25);

  return {
    priorityFeeUsd,
    baseFeeUsd,
    l1DataCostTotalUsd,
    l1ProveCost,
    baseFeeDaoUsd,
  };
};

/**
 * Calculate processed data for a single sequencer
 */
export const calculateSequencerData = (
  sequencerFee: SequencerFeeData,
  ethPrice: number,
  hardwareCostPerSeq: number,
): ProcessedSequencerData => {
  const priorityGwei = sequencerFee.priority_fee ?? 0;
  const baseGwei = (sequencerFee.base_fee ?? 0) * SEQUENCER_BASE_FEE_RATIO;
  const l1CostGwei = sequencerFee.l1_data_cost ?? 0;
  const proveGwei = sequencerFee.prove_cost ?? 0;

  const priorityUsd = safeValue((priorityGwei / GWEI_TO_ETH) * ethPrice);
  const baseUsd = safeValue((baseGwei / GWEI_TO_ETH) * ethPrice);
  const l1CostUsd = safeValue((l1CostGwei / GWEI_TO_ETH) * ethPrice);
  const proveUsd = safeValue((proveGwei / GWEI_TO_ETH) * ethPrice);

  const revenue = safeValue(priorityUsd + baseUsd);
  const revenueGwei = safeValue(priorityGwei + baseGwei);

  // Calculate profit using existing utility
  const { profitUsd, profitEth } = calculateProfit({
    priorityFee: priorityGwei,
    baseFee: sequencerFee.base_fee ?? 0,
    l1DataCost: l1CostGwei,
    proveCost: proveGwei,
    hardwareCostUsd: hardwareCostPerSeq,
    ethPrice,
  });

  const profit = safeValue(profitUsd);
  const profitGwei = safeValue(profitEth * GWEI_TO_ETH);

  // Use full costs for flow accounting; subsidy covers any shortfall
  const actualHardwareCost = hardwareCostPerSeq;
  const actualProveCost = proveUsd;
  const actualL1Cost = l1CostUsd;

  // Calculate subsidy as total costs minus revenue when revenue is insufficient
  const totalCostsUsd = safeValue(
    actualHardwareCost + actualProveCost + actualL1Cost,
  );
  const subsidyUsd = safeValue(Math.max(0, totalCostsUsd - revenue));
  const subsidyGwei = safeValue(
    ethPrice ? (subsidyUsd / ethPrice) * GWEI_TO_ETH : 0,
  );

  // Convert costs to Gwei
  const actualHardwareCostGwei = safeValue(
    ethPrice ? (actualHardwareCost / ethPrice) * GWEI_TO_ETH : 0,
  );
  const actualL1CostGwei = safeValue(
    ethPrice ? (actualL1Cost / ethPrice) * GWEI_TO_ETH : 0,
  );
  const actualProveCostGwei = safeValue(
    ethPrice ? (actualProveCost / ethPrice) * GWEI_TO_ETH : 0,
  );

  const name = getSequencerName(sequencerFee.address);
  const shortAddress =
    name.toLowerCase() === sequencerFee.address.toLowerCase()
      ? sequencerFee.address.slice(0, 7)
      : name;

  return {
    address: sequencerFee.address,
    shortAddress,
    priorityUsd,
    baseUsd,
    revenue,
    revenueGwei,
    profit,
    profitGwei,
    actualHardwareCost,
    actualL1Cost,
    actualProveCost,
    l1CostUsd,
    subsidyUsd,
    subsidyGwei,
    actualHardwareCostGwei,
    actualL1CostGwei,
    actualProveCostGwei,
  };
};

/**
 * Process all sequencer data and sort by profitability
 */
export const processSequencerData = (
  sequencerFees: SequencerFeeData[],
  ethPrice: number,
  hardwareCostPerSeq: number,
): ProcessedSequencerData[] => {
  // First, aggregate raw fee rows by operator name so we can recompute
  // cost allocation and subsidy at the group level (prevents false subsidy
  // when individual addresses are net-negative but the group is net-positive).
  type RawAgg = {
    representative: string; // address to keep for labeling/links
    count: number; // number of addresses in the group
    priority_fee: number;
    base_fee: number;
    l1_data_cost: number;
    prove_cost: number;
  };

  const feesByName = new Map<string, RawAgg>();
  for (const f of sequencerFees) {
    const name = getSequencerName(f.address);
    const existing = feesByName.get(name);
    if (!existing) {
      feesByName.set(name, {
        representative: f.address,
        count: 1,
        priority_fee: f.priority_fee ?? 0,
        base_fee: f.base_fee ?? 0,
        l1_data_cost: f.l1_data_cost ?? 0,
        prove_cost: f.prove_cost ?? 0,
      });
    } else {
      existing.count += 1;
      existing.priority_fee += f.priority_fee ?? 0;
      existing.base_fee += f.base_fee ?? 0;
      existing.l1_data_cost += f.l1_data_cost ?? 0;
      existing.prove_cost += f.prove_cost ?? 0;
    }
  }

  // Recompute per-group processed data using aggregated raw numbers and
  // aggregated hardware costs (per-sequencer cost * number of addresses).
  const consolidatedData: ProcessedSequencerData[] = [];
  for (const [name, agg] of feesByName.entries()) {
    const aggregatedFee: SequencerFeeData = {
      address: agg.representative,
      priority_fee: agg.priority_fee,
      base_fee: agg.base_fee,
      l1_data_cost: agg.l1_data_cost,
      prove_cost: agg.prove_cost,
    };
    const processed = calculateSequencerData(
      aggregatedFee,
      ethPrice,
      hardwareCostPerSeq * Math.max(1, agg.count),
    );
    // Override shortAddress to the operator name for clarity
    processed.shortAddress = name;
    consolidatedData.push(processed);
  }

  // Sort sequencer nodes by profitability (ascending) to reduce flow crossings
  return consolidatedData.sort((a, b) => a.profit - b.profit);
};

/**
 * Generate Sankey chart data for fallback scenario (no sequencer data)
 */
export const generateFallbackSankeyData = ({
  priorityFeeUsd,
  baseFeeUsd,
  baseFeeDaoUsd,
  l1DataCostTotalUsd,
  l1ProveCost,
  totalHardwareCost,
  priorityFee = 0,
  baseFee = 0,
  ethPrice,
}: FallbackCalculationParams): SankeyChartData => {
  const sequencerRevenue = safeValue(priorityFeeUsd + baseFeeUsd * SEQUENCER_BASE_FEE_RATIO);
  const totalCosts = safeValue(totalHardwareCost + l1ProveCost + l1DataCostTotalUsd);
  const l1Subsidy = safeValue(Math.max(0, totalCosts - sequencerRevenue));
  const sequencerProfit = safeValue(Math.max(0, sequencerRevenue - totalCosts));

  const sequencerRevenueGwei = safeValue(
    (priorityFee ?? 0) + (baseFee ?? 0) * SEQUENCER_BASE_FEE_RATIO,
  );
  const sequencerProfitGwei = safeValue(
    ethPrice ? (sequencerProfit / ethPrice) * GWEI_TO_ETH : 0,
  );

  // Define node indices for easier reference
  const daoIndex = 4;
  const hardwareIndex = 5;
  const proveIndex = 6;
  const proposeIndex = 7;
  const profitIndex = 8;

  const nodes: SankeyNode[] = [
    { name: 'Subsidy', value: l1Subsidy, usd: true, depth: 0 },
    {
      name: 'Priority Fee',
      value: priorityFeeUsd,
      gwei: priorityFee ?? 0,
      depth: 0,
    },
    { name: 'Base Fee', value: baseFeeUsd, gwei: baseFee ?? 0, depth: 0 },
    {
      name: 'Sequencers',
      value: sequencerRevenue,
      gwei: sequencerRevenueGwei,
      depth: 1,
    },
    {
      name: 'Taiko DAO',
      value: baseFeeDaoUsd,
      gwei: (baseFee ?? 0) * 0.25,
      depth: 1,
    },
    { name: 'Hardware Cost', value: totalHardwareCost, usd: true, depth: 2 },
    { name: 'Proving Cost', value: l1ProveCost, usd: true, depth: 2 },
    {
      name: 'Proposing Cost',
      value: l1DataCostTotalUsd,
      usd: true,
      depth: 2,
    },
    {
      name: 'Profit',
      value: sequencerProfit,
      gwei: sequencerProfitGwei,
      depth: 3,
    },
  ];

  // Build links with updated indices
  const links: SankeyLink[] = [
    { source: 1, target: 3, value: priorityFeeUsd },
    { source: 2, target: 3, value: safeValue(baseFeeUsd * SEQUENCER_BASE_FEE_RATIO) },
    { source: 2, target: daoIndex, value: baseFeeDaoUsd },
    // Route all costs from Sequencers; subsidy flows into Sequencers
    { source: 3, target: hardwareIndex, value: totalHardwareCost },
    { source: 3, target: proveIndex, value: l1ProveCost },
    { source: 3, target: proposeIndex, value: l1DataCostTotalUsd },
    { source: 0, target: 3, value: l1Subsidy },
    { source: 3, target: profitIndex, value: sequencerProfit },
  ].filter((l) => l.value !== 0);

  // Ensure Taiko DAO is not a sink so it appears in the middle column
  const minPositiveDao = links.length
    ? Math.min(...links.map((l) => l.value))
    : 0;
  const DAO_EPSILON = minPositiveDao > 0 ? minPositiveDao * 0.1 : 1e-6;
  const daoHasOutflow = links.some(
    (l) => l.source === daoIndex && l.value > 0,
  );

  if (!daoHasOutflow) {
    links.push({ source: daoIndex, target: profitIndex, value: DAO_EPSILON });
    if (nodes[profitIndex]) {
      nodes[profitIndex].value += DAO_EPSILON;
    }
  }

  return { nodes, links };
};

/**
 * Generate Sankey chart data for multi-sequencer scenario
 */
export const generateMultiSequencerSankeyData = (
  seqData: ProcessedSequencerData[],
  priorityFeeUsd: number,
  baseFeeUsd: number,
  baseFeeDaoUsd: number,
  priorityFee?: number | null,
  baseFee?: number | null,
): SankeyChartData => {
  const totalActualHardwareCost = seqData.reduce((acc, s) => acc + s.actualHardwareCost, 0);
  const totalActualL1Cost = seqData.reduce((acc, s) => acc + s.actualL1Cost, 0);
  const totalActualProveCost = seqData.reduce((acc, s) => acc + s.actualProveCost, 0);
  const totalSubsidy = seqData.reduce((acc, s) => acc + s.subsidyUsd, 0);
  const totalSubsidyGwei = seqData.reduce((acc, s) => acc + s.subsidyGwei, 0);

  // Aggregate profit across all sequencers (profit may be negative per sequencer)
  const totalProfit = seqData.reduce((acc, s) => acc + s.profit, 0);
  const totalProfitGwei = seqData.reduce((acc, s) => acc + s.profitGwei, 0);

  // Proposing cost equals full L1 cost; subsidy flows into sequencers not L1
  const totalL1Cost = totalActualL1Cost;

  // Build Sankey data with Subsidy node and combined sequencer nodes
  const totalSubsidyIndex = 0;
  const priorityIndex = 1;
  const baseFeeIndex = 2;
  const sequencerStartIndex = 3; // first sequencer node index
  const daoIndex = sequencerStartIndex + seqData.length;
  const hardwareIndex = daoIndex + 1;
  const proveIndex = hardwareIndex + 1;
  const l1Index = hardwareIndex + 2;
  const profitIndex = l1Index + 1;

  const nodes: SankeyNode[] = [
    // Subsidy node at index 0
    {
      name: 'Subsidy',
      value: totalSubsidy,
      gwei: totalSubsidyGwei,
      usd: true,
      depth: 0,
    },
    {
      name: 'Priority Fee',
      value: priorityFeeUsd,
      gwei: priorityFee ?? 0,
      depth: 0,
    },
    { name: 'Base Fee', value: baseFeeUsd, gwei: baseFee ?? 0, depth: 0 },
    // Combined sequencer nodes (revenue + subsidy)
    ...seqData.map((s) => ({
      name: s.shortAddress,
      address: s.address,
      addressLabel: s.shortAddress,
      value: s.revenue + s.subsidyUsd,
      gwei: s.revenueGwei + s.subsidyGwei,
      depth: 1,
    })),
    {
      name: 'Taiko DAO',
      value: baseFeeDaoUsd,
      gwei: (baseFee ?? 0) * 0.25,
      depth: 1,
    },
    {
      name: 'Hardware Cost',
      value: totalActualHardwareCost,
      usd: true,
      depth: 2,
    },
    {
      name: 'Proving Cost',
      value: totalActualProveCost,
      usd: true,
      depth: 2,
    },
    { name: 'Proposing Cost', value: totalL1Cost, usd: true, depth: 2 },
    {
      name: 'Profit',
      value: totalProfit,
      gwei: totalProfitGwei,
      profitNode: true,
      depth: 3,
    },
  ];

  const links: SankeyLink[] = [
    // Subsidy → Sequencer nodes (combined)
    ...seqData.map((s, i) => ({
      source: totalSubsidyIndex,
      target: sequencerStartIndex + i,
      value: s.subsidyUsd,
    })),
    // Priority Fee → Sequencer nodes (combined)
    ...seqData.map((s, i) => ({
      source: priorityIndex,
      target: sequencerStartIndex + i,
      value: s.priorityUsd,
    })),
    // Base Fee → Sequencer nodes (combined)
    ...seqData.map((s, i) => ({
      source: baseFeeIndex,
      target: sequencerStartIndex + i,
      value: s.baseUsd,
    })),
    // Base Fee → Taiko DAO
    { source: baseFeeIndex, target: daoIndex, value: baseFeeDaoUsd },
    // Sequencer nodes → Hardware Cost
    ...seqData.map((s, i) => ({
      source: sequencerStartIndex + i,
      target: hardwareIndex,
      value: s.actualHardwareCost,
    })),
    // Sequencer nodes → Proving Cost
    ...seqData.map((s, i) => ({
      source: sequencerStartIndex + i,
      target: proveIndex,
      value: s.actualProveCost,
    })),
    // Sequencer nodes → Proposing Cost (full L1 cost)
    ...seqData.map((s, i) => ({
      source: sequencerStartIndex + i,
      target: l1Index,
      value: s.actualL1Cost,
    })),
    // Sequencer nodes → Profit (single aggregated node)
    ...seqData.map((s, i) => ({
      source: sequencerStartIndex + i,
      target: profitIndex,
      value: s.profit,
    })),
  ].filter((l) => l.value !== 0);

  // --- Ensure every sequencer node has at least one outgoing edge ---
  // Use 10% of the smallest existing link so the line is always visible
  const minPositive = links.length
    ? Math.min(...links.map((l) => l.value))
    : 0;
  const EPSILON = minPositive > 0 ? minPositive * 0.1 : 1e-6;

  seqData.forEach((_, i) => {
    const seqIdx = sequencerStartIndex + i;
    const hasOutflow = links.some((l) => l.source === seqIdx && l.value > 0);
    if (!hasOutflow) {
      links.push({ source: seqIdx, target: profitIndex, value: EPSILON });
      // keep mass-balance
      if (nodes[profitIndex]) {
        nodes[profitIndex].value += EPSILON;
      }
    }
  });

  // --- Ensure Taiko DAO node has an outgoing edge so it sits with sequencers ---
  const daoHasOutflow2 = links.some(
    (l) => l.source === daoIndex && l.value > 0,
  );
  if (!daoHasOutflow2) {
    links.push({ source: daoIndex, target: profitIndex, value: EPSILON });
    if (nodes[profitIndex]) {
      nodes[profitIndex].value += EPSILON;
    }
  }

  return { nodes, links };
};

/**
 * Validate and filter Sankey chart data
 */
export const validateChartData = (
  chartData: SankeyChartData,
): SankeyChartData => {
  const { nodes, links } = chartData;

  // Additional safety checks before processing
  if (!nodes || !links || nodes.length === 0 || links.length === 0) {
    return { nodes: [], links: [] };
  }

  // Validate that all link indices are within bounds
  const maxNodeIndex = nodes.length - 1;
  const validLinks = links.filter((link) => {
    const sourceValid =
      Number.isInteger(link.source) &&
      link.source >= 0 &&
      link.source <= maxNodeIndex;
    const targetValid =
      Number.isInteger(link.target) &&
      link.target >= 0 &&
      link.target <= maxNodeIndex;
    const valueValid = isFinite(link.value) && link.value > 0;
    return sourceValid && targetValid && valueValid;
  });

  // If we don't have valid links, return empty data
  if (validLinks.length === 0) {
    return { nodes: [], links: [] };
  }

  // Remove nodes that have no remaining links after filtering
  const usedIndices = new Set<number>();
  validLinks.forEach((l) => {
    usedIndices.add(l.source);
    usedIndices.add(l.target);
  });

  const indexMap = new Map<number, number>();
  const filteredNodes = nodes.filter((_, idx) => {
    if (usedIndices.has(idx)) {
      indexMap.set(idx, indexMap.size);
      return true;
    }
    return false;
  });

  // Ensure we have nodes after filtering
  if (filteredNodes.length === 0) {
    return { nodes: [], links: [] };
  }

  const remappedLinks = validLinks.map((l) => ({
    ...l,
    source: indexMap.get(l.source) as number,
    target: indexMap.get(l.target) as number,
  }));

  // Final validation: ensure all values are valid numbers
  const validatedNodes = filteredNodes.map((node) => ({
    ...node,
    value: safeValue(node.value),
    gwei: node.gwei ? safeValue(node.gwei) : node.gwei,
  }));

  const validatedLinks = remappedLinks
    .map((link) => ({
      ...link,
      value: safeValue(link.value),
    }))
    .filter((link) => link.value > 0 && isFinite(link.value));

  // Final check to ensure we have valid data
  if (validatedNodes.length === 0 || validatedLinks.length === 0) {
    return { nodes: [], links: [] };
  }

  return { nodes: validatedNodes, links: validatedLinks };
};