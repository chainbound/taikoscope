import React from 'react';
import { ResponsiveContainer, Sankey, Tooltip } from 'recharts';
import type { TooltipProps } from 'recharts';
import { formatEth } from '../utils';
import { TAIKO_PINK, lightTheme, darkTheme } from '../theme';
import { useTheme } from '../contexts/ThemeContext';
import { calculateProfit } from '../utils/profit';

const NODE_GREEN = '#22c55e';
import useSWR from 'swr';
import { fetchL2FeesComponents } from '../services/apiService';
import { getSequencerName } from '../sequencerConfig';
import { useEthPrice } from '../services/priceService';
import { TimeRange } from '../types';
import { rangeToHours } from '../utils/timeRange';
import { calculateHardwareCost } from '../utils/hardwareCost';

interface FeeFlowChartProps {
  timeRange: TimeRange;
  cloudCost: number;
  proverCost: number;
  address?: string;
  /** Height of the chart in pixels (defaults to 320) */
  height?: number;
  /** Total number of sequencers (used for hardware cost scaling) */
  totalSequencers?: number;
}

const WEI_TO_ETH = 1e9;

// Format numbers as USD without grouping
const formatUsd = (value: number) => `$${value.toFixed(1)}`;

// Simple node component that renders label with currency-aware value
const createSankeyNode = (
  textColor: string,
  formatValue: (value: number, itemData?: any) => string,
) => {
  const SankeyNodeComponent = ({ x, y, width, height, payload }: any) => {
    // Guard against NaN values
    const safeX = isNaN(x) ? 0 : x;
    const safeY = isNaN(y) ? 0 : y;
    const safeWidth = isNaN(width) ? 0 : width;
    const safeHeight = isNaN(height) ? 0 : height;

    // Constants for centering the combined label block
    const LINE_HEIGHT = 12;           // More conservative estimate for 12px font
    const NUM_LINES = 2;              // name + value
    const blockHalf = (LINE_HEIGHT * (NUM_LINES - 1)) / 2;  // = 6 px

    const isCostNode =
      payload.name === 'Hardware Cost' ||
      payload.name === 'Proposing Cost' ||
      payload.name === 'Proving Cost';
    const isSubsidyNode = payload.name === 'Subsidy' || (typeof payload.name === 'string' && payload.name.includes('Subsidy'));
    const isProfitNode = payload.name === 'Profit' || payload.profitNode;
    const isPinkNode = payload.name === 'Taiko DAO';
    const hideLabel = payload.hideLabel;
    const addressLabel = payload.addressLabel;

    let label = addressLabel ?? payload.name;
    if (isProfitNode && addressLabel) {
      label = `${addressLabel} Profit`;
    } else if (payload.revenueNode && addressLabel) {
      label = `${addressLabel} Revenue`;
    } else if (payload.subsidyNode && addressLabel) {
      label = `${addressLabel} Subsidy`;
    }

    return (
      <g>
        <rect
          x={safeX}
          y={safeY}
          width={safeWidth}
          height={safeHeight}
          fill={isCostNode ? '#ef4444' : isPinkNode ? TAIKO_PINK : isSubsidyNode ? NODE_GREEN : NODE_GREEN}
          fillOpacity={0.8}
        />
        {!hideLabel && (
          <text
            x={safeX + safeWidth + 6}
            y={safeY + safeHeight / 2 - blockHalf}
            textAnchor="start"
            dominantBaseline="middle"
            fontSize={12}
            fill={textColor}
          >
            <tspan x={safeX + safeWidth + 6}>{label}</tspan>
            <tspan x={safeX + safeWidth + 6} dy="1.2em">
              {formatValue(payload.value, payload)}
            </tspan>
          </text>
        )}
      </g>
    );
  };

  SankeyNodeComponent.displayName = 'SankeyNode';
  return SankeyNodeComponent;
};

const SankeyLink = (props: any) => {
  const {
    sourceX,
    sourceY,
    sourceControlX,
    targetX,
    targetY,
    targetControlX,
    linkWidth,
    payload,
    // Remove props that shouldn't be passed to DOM elements
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    sourceRelativeY,
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    targetRelativeY,
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    index,
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    isAnimationActive,
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    animationBegin,
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    animationDuration,
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    animationEasing,
    ...domProps
  } = props;

  // Guard against NaN values in coordinates
  const safeSourceX = isNaN(sourceX) ? 0 : sourceX;
  const safeSourceY = isNaN(sourceY) ? 0 : sourceY;
  const safeSourceControlX = isNaN(sourceControlX) ? 0 : sourceControlX;
  const safeTargetX = isNaN(targetX) ? 0 : targetX;
  const safeTargetY = isNaN(targetY) ? 0 : targetY;
  const safeTargetControlX = isNaN(targetControlX) ? 0 : targetControlX;
  // Use the link width provided by Recharts without overriding
  const safeLinkWidth = isNaN(linkWidth) ? 0 : linkWidth;

  const isCost =
    payload.target.name === 'Hardware Cost' ||
    payload.target.name === 'Proposing Cost' ||
    payload.target.name === 'Proving Cost' ||

    payload.target.name === 'Subsidy' ||
    (typeof payload.target.name === 'string' &&
      payload.target.name.includes('Subsidy'));
  const isProfit =
    payload.target.name === 'Profit' || payload.target.profitNode;

  return (
    <path
      className="recharts-sankey-link"
      d={`M${safeSourceX},${safeSourceY}C${safeSourceControlX},${safeSourceY} ${safeTargetControlX},${safeTargetY} ${safeTargetX},${safeTargetY}`}
      fill="none"
      stroke={isCost ? '#ef4444' : isProfit ? NODE_GREEN : '#94a3b8'}
      strokeWidth={safeLinkWidth}
      strokeOpacity={0.2}
      {...domProps}
    />
  );
};

export const FeeFlowChart: React.FC<FeeFlowChartProps> = ({
  timeRange,
  cloudCost,
  proverCost,
  address,
  height = 480,
  totalSequencers,
}) => {
  const { theme } = useTheme();
  const textColor =
    theme === 'dark' ? darkTheme.foreground : lightTheme.foreground;
  const { data: feeRes } = useSWR(['l2FeesFlow', timeRange, address], () =>
    fetchL2FeesComponents(timeRange),
  );
  const { data: ethPrice = 0 } = useEthPrice();

  const priorityFee = feeRes?.data?.priority_fee ?? null;
  const baseFee = feeRes?.data?.base_fee ?? null;
  const allSequencerFees = feeRes?.data?.sequencers ?? [];
  const sequencerFees = address 
    ? allSequencerFees.filter(s => s.address.toLowerCase() === address.toLowerCase())
    : allSequencerFees;

  // Memoized tooltip value formatter to avoid unnecessary re-renders
  // NOTE: Depends on `ethPrice`, so it is recreated only when the price changes
  const formatTooltipValue = React.useCallback(
    (value: number, itemData?: any) => {
      const usd = formatUsd(value);

      // If the item already has a `wei` value, use it directly
      if (itemData?.wei != null) {
        return `${formatEth(itemData.wei, 4)} (${usd})`;
      }

      // Otherwise, attempt to derive `wei` from USD using the current ETH price
      if (ethPrice) {
        const wei = (value / ethPrice) * WEI_TO_ETH;
        return `${formatEth(wei, 4)} (${usd})`;
      }

      // Fallback (should rarely happen): return USD only
      return usd;
    },
    [ethPrice],
  );

  // Node value formatter - shows only ETH values without USD
  const formatNodeValue = React.useCallback(
    (value: number, itemData?: any) => {
      // If the item already has a `wei` value, use it directly
      if (itemData?.wei != null) {
        return formatEth(itemData.wei, 4);
      }

      // Otherwise, attempt to derive `wei` from USD using the current ETH price
      if (ethPrice) {
        const wei = (value / ethPrice) * WEI_TO_ETH;
        return formatEth(wei, 4);
      }

      // Fallback (should rarely happen): return USD only
      return formatUsd(value);
    },
    [ethPrice],
  );

  const NodeComponent = React.useMemo(
    () => createSankeyNode(textColor, formatNodeValue),
    [textColor, formatNodeValue],
  );

  if (!feeRes) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        Loading...
      </div>
    );
  }
  if (priorityFee == null && baseFee == null) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }

  // Guard against invalid ethPrice that could cause NaN
  if (!ethPrice || isNaN(ethPrice) || ethPrice <= 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        ETH price unavailable
      </div>
    );
  }

  // Helper function to ensure finite values
  const safeValue = (value: number) => (isFinite(value) ? value : 0);

  // Convert fees to USD
  const priorityFeeUsd = safeValue(((priorityFee ?? 0) / WEI_TO_ETH) * ethPrice);
  const baseFeeUsd = safeValue(((baseFee ?? 0) / WEI_TO_ETH) * ethPrice);
  const l1DataCostTotalUsd = safeValue(
    ((feeRes?.data?.l1_data_cost ?? 0) / WEI_TO_ETH) * ethPrice,
  );
  const l1ProveCost = safeValue(
    ((feeRes?.data?.prove_cost ?? 0) / WEI_TO_ETH) * ethPrice,
  );

  const baseFeeDaoUsd = safeValue(baseFeeUsd * 0.25);

  // Scale operational costs to the selected time range
  const hours = rangeToHours(timeRange);
  const sequencerCount = Math.max(1, totalSequencers ?? sequencerFees.length);
  const {
    totalUsd: rawTotalHardwareCost,
    perSequencerUsd: rawHardwareCostPerSeq,
  } = calculateHardwareCost(cloudCost, proverCost, sequencerCount, hours);
  const totalHardwareCost = safeValue(rawTotalHardwareCost);
  const hardwareCostPerSeq = safeValue(rawHardwareCostPerSeq);

  const seqData = sequencerFees.map((f) => {
    const priorityWei = f.priority_fee ?? 0;
    const baseWei = (f.base_fee ?? 0) * 0.75;
    const l1CostWei = f.l1_data_cost ?? 0;
    const proveWei = f.prove_cost ?? 0;

    const priorityUsd = safeValue((priorityWei / WEI_TO_ETH) * ethPrice);
    const baseUsd = safeValue((baseWei / WEI_TO_ETH) * ethPrice);
    const l1CostUsd = safeValue((l1CostWei / WEI_TO_ETH) * ethPrice);
    const proveUsd = safeValue((proveWei / WEI_TO_ETH) * ethPrice);


    const revenue = safeValue(priorityUsd + baseUsd);
    const revenueWei = safeValue(priorityWei + baseWei);

    const { profitUsd, profitEth } = calculateProfit({
      priorityFee: priorityWei,
      baseFee: f.base_fee ?? 0,
      l1DataCost: l1CostWei,
      proveCost: proveWei,

      hardwareCostUsd: hardwareCostPerSeq,
      ethPrice,
    });
    const profit = safeValue(profitUsd);
    const profitWei = safeValue(profitEth * WEI_TO_ETH);
    let remaining = revenue;
    // Always allocate full hardware cost share per sequencer (sum will equal totalHardwareCost)
    const actualHardwareCost = hardwareCostPerSeq;
    remaining -= actualHardwareCost;
    const actualProveCost = safeValue(Math.min(proveUsd, remaining));
    remaining -= actualProveCost;
    const actualL1Cost = safeValue(Math.min(l1CostUsd, remaining));
    remaining -= actualL1Cost;
    const deficitUsd = safeValue(Math.max(0, -profitUsd));
    const subsidyUsd = safeValue(Math.max(l1CostUsd - actualL1Cost, deficitUsd));
    const subsidyWei = safeValue(
      ethPrice ? (subsidyUsd / ethPrice) * WEI_TO_ETH : 0,
    );
    const actualHardwareCostWei = safeValue(
      ethPrice ? (actualHardwareCost / ethPrice) * WEI_TO_ETH : 0,
    );
    const actualL1CostWei = safeValue(
      ethPrice ? (actualL1Cost / ethPrice) * WEI_TO_ETH : 0,
    );
    const actualProveCostWei = safeValue(
      ethPrice ? (actualProveCost / ethPrice) * WEI_TO_ETH : 0,
    );

    const name = getSequencerName(f.address);
    const shortAddress =
      name.toLowerCase() === f.address.toLowerCase()
        ? f.address.slice(0, 7)
        : name;
    return {
      address: f.address,
      shortAddress,
      priorityUsd,
      baseUsd,
      revenue,
      revenueWei,
      profit,
      profitWei,
      actualHardwareCost,
      actualL1Cost,
      actualProveCost,

      l1CostUsd,
      subsidyUsd,
      subsidyWei,
      actualHardwareCostWei,
      actualL1CostWei,
      actualProveCostWei,

    };
  });

  // Sort sequencer nodes by profitability (ascending) to reduce flow crossings
  seqData.sort((a, b) => a.profit - b.profit);

  // Handle case when no sequencer data is available
  let nodes: any[], links: { source: number; target: number; value: number }[];

  if (seqData.length === 0) {
    // Fallback: create a single "Sequencers" node to route fees through
    const sequencerRevenue = safeValue(priorityFeeUsd + baseFeeUsd * 0.75);
    let remaining = sequencerRevenue - totalHardwareCost;
    const actualProveCost = safeValue(Math.min(l1ProveCost, Math.max(0, remaining)));
    remaining -= actualProveCost;
    const actualL1Cost = safeValue(Math.min(l1DataCostTotalUsd, Math.max(0, remaining)));
    remaining -= actualL1Cost;
    const l1Subsidy = safeValue(l1DataCostTotalUsd - actualL1Cost);
    const sequencerProfit = safeValue(Math.max(0, remaining));
    const sequencerRevenueWei = safeValue((priorityFee ?? 0) + (baseFee ?? 0) * 0.75);
    const sequencerProfitWei = safeValue(
      ethPrice ? (sequencerProfit / ethPrice) * WEI_TO_ETH : 0,
    );

    // Define node indices for easier reference
    const daoIndex = 4;
    const hardwareIndex = 5;
    const proveIndex = 6;
    const proposeIndex = 7;
    const profitIndex = 8;

    nodes = [
      { name: 'Subsidy', value: l1Subsidy, usd: true, depth: 0 },
      { name: 'Priority Fee', value: priorityFeeUsd, wei: priorityFee ?? 0, depth: 0 },
      { name: 'Base Fee', value: baseFeeUsd, wei: baseFee ?? 0, depth: 0 },
      { name: 'Sequencers', value: sequencerRevenue, wei: sequencerRevenueWei, depth: 1 },
      { name: 'Taiko DAO', value: baseFeeDaoUsd, wei: (baseFee ?? 0) * 0.25, depth: 1 },
      { name: 'Hardware Cost', value: totalHardwareCost, usd: true, depth: 2 },
      { name: 'Proving Cost', value: l1ProveCost, usd: true, depth: 2 },
      { name: 'Proposing Cost', value: l1DataCostTotalUsd, usd: true, depth: 2 },
      { name: 'Profit', value: sequencerProfit, wei: sequencerProfitWei, depth: 3 },
    ];

    // Build links with updated indices
    links = [
      { source: 1, target: 3, value: priorityFeeUsd },
      { source: 2, target: 3, value: safeValue(baseFeeUsd * 0.75) },
      { source: 2, target: daoIndex, value: baseFeeDaoUsd },
      { source: 3, target: hardwareIndex, value: safeValue(Math.min(totalHardwareCost, sequencerRevenue)) },
      { source: 3, target: proveIndex, value: l1ProveCost },
      { source: 3, target: proposeIndex, value: actualL1Cost },
      { source: 0, target: proposeIndex, value: l1Subsidy },
      { source: 3, target: profitIndex, value: sequencerProfit },
    ].filter((l) => l.value !== 0);

    // Ensure Taiko DAO is not a sink so it appears in the middle column
    const minPositiveDao = links.length ? Math.min(...links.map(l => l.value)) : 0;
    const DAO_EPSILON = minPositiveDao > 0 ? minPositiveDao * 0.1 : 1e-6;
    const daoHasOutflow = links.some((l) => l.source === daoIndex && l.value > 0);
    if (!daoHasOutflow) {
      links.push({ source: daoIndex, target: profitIndex, value: DAO_EPSILON });
      if (nodes[profitIndex]) {
        nodes[profitIndex].value += DAO_EPSILON;
      }
    }

  } else {
    const totalActualHardwareCost = seqData.reduce(
      (acc, s) => acc + s.actualHardwareCost,
      0,
    );
    const totalActualL1Cost = seqData.reduce(
      (acc, s) => acc + s.actualL1Cost,
      0,
    );
    const totalSubsidy = seqData.reduce((acc, s) => acc + s.subsidyUsd, 0);
    const totalSubsidyWei = seqData.reduce((acc, s) => acc + s.subsidyWei, 0);
    const totalActualProveCost = seqData.reduce(
      (acc, s) => acc + s.actualProveCost,
      0,
    );

    // Aggregate profit across all sequencers
    const totalProfit = seqData.reduce((acc, s) => acc + s.profit, 0);
    const totalProfitWei = seqData.reduce((acc, s) => acc + s.profitWei, 0);

    const totalL1Cost = totalActualL1Cost + totalSubsidy;

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

    nodes = [
      // Subsidy node at index 0
      {
        name: 'Subsidy',
        value: totalSubsidy,
        wei: totalSubsidyWei,
        usd: true,
        depth: 0,
      },
      { name: 'Priority Fee', value: priorityFeeUsd, wei: priorityFee ?? 0, depth: 0 },
      { name: 'Base Fee', value: baseFeeUsd, wei: baseFee ?? 0, depth: 0 },
      // Combined sequencer nodes (revenue + subsidy)
      ...seqData.map((s) => ({
        name: s.shortAddress,
        address: s.address,
        addressLabel: s.shortAddress,
        value: s.revenue + s.subsidyUsd,
        wei: s.revenueWei + s.subsidyWei,
        depth: 1,
      })),
      { name: 'Taiko DAO', value: baseFeeDaoUsd, wei: (baseFee ?? 0) * 0.25, depth: 1 },
      { name: 'Hardware Cost', value: totalActualHardwareCost, usd: true, depth: 2 },
      { name: 'Proving Cost', value: totalActualProveCost, usd: true, depth: 2 },
      { name: 'Proposing Cost', value: totalL1Cost, usd: true, depth: 2 },
      {
        name: 'Profit',
        value: totalProfit,
        wei: totalProfitWei,
        profitNode: true,
        depth: 3,
      },
    ];

    links = [
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
      // Sequencer nodes → Proposing Cost
      ...seqData.map((s, i) => ({
        source: sequencerStartIndex + i,
        target: l1Index,
        value: s.l1CostUsd,
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
    const minPositive = links.length ? Math.min(...links.map(l => l.value)) : 0;
    const EPSILON = minPositive > 0 ? minPositive * 0.1 : 1e-6;
    seqData.forEach((_, i) => {
      const seqIdx = sequencerStartIndex + i;
      const hasOutflow = links.some(
        (l) => l.source === seqIdx && l.value > 0,
      );
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

  }

  // Additional safety checks before processing
  if (!nodes || !links || nodes.length === 0 || links.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        Insufficient data for flow chart
      </div>
    );
  }

  // Validate that all link indices are within bounds
  const maxNodeIndex = nodes.length - 1;
  const validLinks = links.filter((link) => {
    const sourceValid = Number.isInteger(link.source) && link.source >= 0 && link.source <= maxNodeIndex;
    const targetValid = Number.isInteger(link.target) && link.target >= 0 && link.target <= maxNodeIndex;
    const valueValid = isFinite(link.value) && link.value > 0;
    return sourceValid && targetValid && valueValid;
  });

  // If we don't have valid links, don't render the chart
  if (validLinks.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        Unable to create flow chart with current data
      </div>
    );
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
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No valid nodes for flow chart
      </div>
    );
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
    wei: (node as any).wei ? safeValue((node as any).wei) : (node as any).wei,
  }));

  const validatedLinks = remappedLinks
    .map((link) => ({
      ...link,
      value: safeValue(link.value),
    }))
    .filter((link) => link.value > 0 && isFinite(link.value));

  // Final check to ensure we have valid data
  if (validatedNodes.length === 0 || validatedLinks.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        Invalid chart data structure
      </div>
    );
  }

  const data = { nodes: validatedNodes, links: validatedLinks };

  const tooltipContent = ({ active, payload }: TooltipProps<number, string>) => {
    if (!active || !payload?.[0]) return null;

    const { value = 0, payload: itemData } = payload![0];

    // Suppress tooltip for flows (links) – they now display values directly on the chart
    if (itemData.source != null && itemData.target != null) {
      return null;
    }

    const nodeLabel = (() => {
      if (itemData.profitNode && itemData.addressLabel) {
        return `${itemData.addressLabel} Profit`;
      }
      return itemData.addressLabel ?? itemData.address ?? itemData.name;
    })();
    return (
      <div className="bg-white dark:bg-gray-800 p-2 border border-gray-200 dark:border-gray-700 rounded shadow-sm">
        <p className="text-sm font-medium">{nodeLabel}</p>
        <p className="text-sm text-gray-600 dark:text-gray-300">
          {formatTooltipValue(value, itemData)}
        </p>
      </div>
    );
  };

  return (
    <div className="mt-6" style={{ height }}>
      <ResponsiveContainer width="100%" height="100%">
        <Sankey
          data={data}
          node={NodeComponent}
          nodePadding={30}
          nodeWidth={10}
          margin={{ top: 20, right: 120, bottom: 20, left: 10 }}
          sort={false}
          iterations={32}
          link={SankeyLink}
        >
          <Tooltip
            content={tooltipContent}
            trigger="hover"
            contentStyle={{
              backgroundColor: theme === 'dark' ? '#1e293b' : 'white',
              border:
                theme === 'dark' ? '1px solid #334155' : '1px solid #e5e7eb',
              borderRadius: '0.375rem',
            }}
          />
        </Sankey>
      </ResponsiveContainer>
    </div>
  );
};
