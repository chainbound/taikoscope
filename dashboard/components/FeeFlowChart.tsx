import React from 'react';
import { ResponsiveContainer, Sankey, Tooltip } from 'recharts';
import type { TooltipProps } from 'recharts';
import { formatEth } from '../utils';
import { TAIKO_PINK, lightTheme, darkTheme } from '../theme';
import { useTheme } from '../contexts/ThemeContext';
import { calculateProfit } from '../utils/profit';

const NODE_GREEN = '#22c55e';
import useSWR from 'swr';
import { fetchBatchFeeComponents } from '../services/apiService';
import { getSequencerName } from '../sequencerConfig';
import { useEthPrice } from '../services/priceService';
import { TimeRange } from '../types';
import { rangeToHours } from '../utils/timeRange';

interface FeeFlowChartProps {
  timeRange: TimeRange;
  cloudCost: number;
  proverCost: number;
  address?: string;
}

const MONTH_HOURS = 30 * 24;
const WEI_TO_ETH = 1e18;

// Format numbers as USD without grouping
const formatUsd = (value: number) => `$${value.toFixed(2)}`;

// Simple node component that renders label with currency-aware value
const createSankeyNode = (textColor: string) => {
  const SankeyNodeComponent = ({ x, y, width, height, payload }: any) => {
    // Guard against NaN values
    const safeX = isNaN(x) ? 0 : x;
    const safeY = isNaN(y) ? 0 : y;
    const safeWidth = isNaN(width) ? 0 : width;
    const safeHeight = isNaN(height) ? 0 : height;

    const isCostNode =
      payload.name === 'Hardware Cost' ||
      payload.name === 'L1 Data Cost' ||
      payload.name === 'L1 Prove Cost' ||
      payload.name === 'L1 Verify Cost' ||
      payload.name === 'Subsidy' ||
      (typeof payload.name === 'string' && payload.name.includes('Subsidy'));
    const isProfitNode = payload.name === 'Profit' || payload.profitNode;
    const isPinkNode = payload.name === 'Taiko DAO';
    const hideLabel = payload.hideLabel;
    const addressLabel = payload.addressLabel;

    let label = addressLabel ?? payload.name;
    if (isProfitNode && addressLabel) {
      label = `${addressLabel} Profit`;
    } else if (payload.incomeNode && addressLabel) {
      label = `${addressLabel} Income`;
    }

    return (
      <g>
        <rect
          x={safeX}
          y={safeY}
          width={safeWidth}
          height={safeHeight}
          fill={isCostNode ? '#ef4444' : isPinkNode ? TAIKO_PINK : NODE_GREEN}
          fillOpacity={0.8}
        />
        {!hideLabel && (
          <text
            x={safeX + safeWidth + 6}
            y={safeY + safeHeight / 2}
            textAnchor="start"
            dominantBaseline="middle"
            fontSize={12}
            fill={textColor}
          >
            {label}
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
  const safeLinkWidth = isNaN(linkWidth) ? 0 : linkWidth;

  const isCost =
    payload.target.name === 'Hardware Cost' ||
    payload.target.name === 'L1 Data Cost' ||
    payload.target.name === 'L1 Prove Cost' ||
    payload.target.name === 'L1 Verify Cost' ||
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
}) => {
  const { theme } = useTheme();
  const textColor =
    theme === 'dark' ? darkTheme.foreground : lightTheme.foreground;
  const { data: batchRes } = useSWR(['feeFlowBatches', timeRange, address], () =>
    fetchBatchFeeComponents(timeRange, address),
  );
  const feeRes = React.useMemo(() => {
    if (!batchRes?.data) return null;
    const map = new Map<string, {
      priority: number;
      base: number;
      l1: number;
      prove: number;
      verify: number;
      count: number;
    }>();
    batchRes.data.forEach((b) => {
      const entry = map.get(b.sequencer) || {
        priority: 0,
        base: 0,
        l1: 0,
        prove: 0,
        verify: 0,
        count: 0,
      };
      entry.priority += b.priority;
      entry.base += b.base;
      entry.l1 += b.l1Cost ?? 0;
      entry.prove += b.amortizedProveCost ?? 0;
      entry.verify += b.amortizedVerifyCost ?? 0;
      entry.count += 1;
      map.set(b.sequencer, entry);
    });
    return { map };
  }, [batchRes]);
  const { data: ethPrice = 0 } = useEthPrice();

  const priorityFee = React.useMemo(() => {
    if (!feeRes) return null;
    let v = 0;
    feeRes.map.forEach((s) => {
      v += s.priority;
    });
    return v;
  }, [feeRes]);
  const baseFee = React.useMemo(() => {
    if (!feeRes) return null;
    let v = 0;
    feeRes.map.forEach((s) => {
      v += s.base;
    });
    return v;
  }, [feeRes]);
  const l1DataCostTotal = React.useMemo(() => {
    if (!feeRes) return 0;
    let v = 0;
    feeRes.map.forEach((s) => {
      v += s.l1;
    });
    return v;
  }, [feeRes]);
  const proveCostTotal = React.useMemo(() => {
    if (!feeRes) return 0;
    let v = 0;
    feeRes.map.forEach((s) => {
      v += s.prove;
    });
    return v;
  }, [feeRes]);
  const verifyCostTotal = React.useMemo(() => {
    if (!feeRes) return 0;
    let v = 0;
    feeRes.map.forEach((s) => {
      v += s.verify;
    });
    return v;
  }, [feeRes]);
  const sequencerFees = React.useMemo(() => {
    if (!feeRes)
      return [] as {
        address: string;
        priority: number;
        base: number;
        l1: number;
        prove: number;
        verify: number;
        count: number;
      }[];
    const arr: { address: string; priority: number; base: number; l1: number; prove: number; verify: number; count: number }[] = [];
    feeRes.map.forEach((v, addr) => {
      arr.push({ address: addr, ...v });
    });
    return arr;
  }, [feeRes]);

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
  const l1DataCostTotalUsd = safeValue((l1DataCostTotal / WEI_TO_ETH) * ethPrice);
  const l1ProveCost = safeValue((proveCostTotal / WEI_TO_ETH) * ethPrice);
  const l1VerifyCost = safeValue((verifyCostTotal / WEI_TO_ETH) * ethPrice);
  const baseFeeDaoUsd = safeValue(baseFeeUsd * 0.25);

  // Scale operational costs to the selected time range
  const hours = rangeToHours(timeRange);
  const totalBatchCount = sequencerFees.reduce((acc, s) => acc + s.count, 0);
  const hardwarePerBatchUsd =
    totalBatchCount > 0
      ? ((cloudCost + proverCost) / MONTH_HOURS) * (hours / totalBatchCount)
      : 0;
  const totalHardwareCost = hardwarePerBatchUsd * totalBatchCount;

  const seqData = sequencerFees.map((f) => {
    const priorityWei = f.priority;
    const baseWei = f.base * 0.75;
    const l1CostWei = f.l1;
    const proveWei = f.prove;
    const verifyWei = f.verify;
    const hardwareCostSeqUsd = hardwarePerBatchUsd * f.count;
    const priorityUsd = safeValue((priorityWei / WEI_TO_ETH) * ethPrice);
    const baseUsd = safeValue((baseWei / WEI_TO_ETH) * ethPrice);
    const l1CostUsd = safeValue((l1CostWei / WEI_TO_ETH) * ethPrice);
    const proveUsd = safeValue((proveWei / WEI_TO_ETH) * ethPrice);
    const verifyUsd = safeValue((verifyWei / WEI_TO_ETH) * ethPrice);

    const revenue = safeValue(priorityUsd + baseUsd);
    const revenueWei = safeValue(priorityWei + baseWei);

    const { profitUsd, profitEth } = calculateProfit({
      priorityFee: priorityWei,
      baseFee: f.base,
      l1DataCost: l1CostWei,
      proveCost: proveWei,
      verifyCost: verifyWei,
      hardwareCostUsd: hardwareCostSeqUsd,
      ethPrice,
    });
    const profit = safeValue(Math.max(0, profitUsd));
    const profitWei = safeValue(profitEth * WEI_TO_ETH);
    let remaining = revenue;
    const actualHardwareCost = safeValue(Math.min(hardwareCostSeqUsd, remaining));
    remaining -= actualHardwareCost;
    const actualL1Cost = safeValue(Math.min(l1CostUsd, remaining));
    remaining -= actualL1Cost;
    const actualProveCost = safeValue(Math.min(proveUsd, remaining));
    remaining -= actualProveCost;
    const actualVerifyCost = safeValue(Math.min(verifyUsd, remaining));
    remaining -= actualVerifyCost;
    const subsidyUsd = safeValue(l1CostUsd - actualL1Cost);
    const subsidyWei = safeValue(ethPrice ? (subsidyUsd / ethPrice) * WEI_TO_ETH : 0);
    const actualHardwareCostWei = safeValue(
      ethPrice ? (actualHardwareCost / ethPrice) * WEI_TO_ETH : 0,
    );
    const actualL1CostWei = safeValue(
      ethPrice ? (actualL1Cost / ethPrice) * WEI_TO_ETH : 0,
    );
    const actualProveCostWei = safeValue(
      ethPrice ? (actualProveCost / ethPrice) * WEI_TO_ETH : 0,
    );
    const actualVerifyCostWei = safeValue(
      ethPrice ? (actualVerifyCost / ethPrice) * WEI_TO_ETH : 0,
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
      actualVerifyCost,
      l1CostUsd,
      subsidyUsd,
      subsidyWei,
      actualHardwareCostWei,
      actualL1CostWei,
      actualProveCostWei,
      actualVerifyCostWei,
    };
  });

  // Handle case when no sequencer data is available
  let nodes, links;

  if (seqData.length === 0) {
    // Fallback: create a single "Sequencers" node to route fees through
    const sequencerRevenue = safeValue(priorityFeeUsd + baseFeeUsd * 0.75);
    let remaining = sequencerRevenue - totalHardwareCost;
    const actualL1Cost = safeValue(Math.min(l1DataCostTotalUsd, Math.max(0, remaining)));
    remaining -= actualL1Cost;
    const l1Subsidy = safeValue(l1DataCostTotalUsd - actualL1Cost);
    const actualProveCost = safeValue(Math.min(l1ProveCost, Math.max(0, remaining)));
    remaining -= actualProveCost;
    const actualVerifyCost = safeValue(Math.min(l1VerifyCost, Math.max(0, remaining)));
    remaining -= actualVerifyCost;
    const sequencerProfit = safeValue(Math.max(0, remaining));
    const sequencerRevenueWei = safeValue((priorityFee ?? 0) + (baseFee ?? 0) * 0.75);
    const sequencerProfitWei = safeValue(
      ethPrice ? (sequencerProfit / ethPrice) * WEI_TO_ETH : 0,
    );

    nodes = [
      { name: 'Subsidy', value: l1Subsidy, usd: true },
      { name: 'Priority Fee', value: priorityFeeUsd, wei: priorityFee ?? 0 },
      { name: 'Base Fee', value: baseFeeUsd, wei: baseFee ?? 0 },
      { name: 'Sequencers', value: sequencerRevenue, wei: sequencerRevenueWei },
      { name: 'Hardware Cost', value: totalHardwareCost, usd: true },
      { name: 'L1 Data Cost', value: l1DataCostTotalUsd, usd: true },
      { name: 'Profit', value: sequencerProfit, wei: sequencerProfitWei },
      { name: 'Taiko DAO', value: baseFeeDaoUsd, wei: (baseFee ?? 0) * 0.25 },
    ];

    let inserted = 0;
    let proveIndex = -1;
    let verifyIndex = -1;
    if (l1ProveCost > 0) {
      proveIndex = 6 + inserted;
      nodes.splice(proveIndex, 0, {
        name: 'L1 Prove Cost',
        value: actualProveCost,
        usd: true,
      });
      inserted += 1;
    }
    if (l1VerifyCost > 0) {
      verifyIndex = 6 + inserted;
      nodes.splice(verifyIndex, 0, {
        name: 'L1 Verify Cost',
        value: actualVerifyCost,
        usd: true,
      });
      inserted += 1;
    }

    const profitIndex = 6 + inserted;
    const daoIndex = profitIndex + 1;

    links = [
      { source: 1, target: 3, value: priorityFeeUsd }, // Priority Fee → Sequencers
      { source: 2, target: 3, value: safeValue(baseFeeUsd * 0.75) }, // 75% Base Fee → Sequencers
      { source: 2, target: daoIndex, value: baseFeeDaoUsd }, // 25% Base Fee → Taiko DAO
      {
        source: 3,
        target: 4,
        value: safeValue(Math.min(totalHardwareCost, sequencerRevenue)),
      }, // Sequencers → Hardware Cost
      {
        source: 3,
        target: 5,
        value: safeValue(Math.min(
          l1DataCostTotalUsd,
          Math.max(0, sequencerRevenue - totalHardwareCost),
        )),
      }, // Sequencers → L1 Data Cost
      { source: 0, target: 5, value: l1Subsidy }, // Subsidy → L1 Data Cost
      { source: 3, target: profitIndex, value: sequencerProfit }, // Sequencers → Profit
    ].filter((l) => l.value > 0);

    if (l1ProveCost > 0 && proveIndex >= 0) {
      links.push({ source: 3, target: proveIndex, value: actualProveCost });
    }
    if (l1VerifyCost > 0 && verifyIndex >= 0) {
      links.push({ source: 3, target: verifyIndex, value: actualVerifyCost });
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
    const totalActualProveCost = seqData.reduce(
      (acc, s) => acc + s.actualProveCost,
      0,
    );
    const totalActualVerifyCost = seqData.reduce(
      (acc, s) => acc + s.actualVerifyCost,
      0,
    );
    const totalL1Cost = totalActualL1Cost + totalSubsidy;

    // Build Sankey data with one node per sequencer
    const subsidyStartIndex = 0;
    const priorityIndex = subsidyStartIndex + seqData.length;
    const baseFeeIndex = priorityIndex + 1;
    const baseIndex = baseFeeIndex + 1; // first sequencer node index
    const hardwareIndex = baseIndex + seqData.length;
    const l1Index = hardwareIndex + 1;
    let profitStartIndex = l1Index + 1;
    let daoIndex = profitStartIndex + seqData.length;

    nodes = [
      ...seqData.map((s) => ({
        // use a unique name per sequencer so nodes don't get aggregated
        name: `${s.shortAddress} Subsidy`,
        address: s.address,
        addressLabel: `${s.shortAddress} Subsidy`,
        value: s.subsidyUsd,
        usd: true,
      })),
      { name: 'Priority Fee', value: priorityFeeUsd, wei: priorityFee ?? 0 },
      { name: 'Base Fee', value: baseFeeUsd, wei: baseFee ?? 0 },
      ...seqData.map((s) => ({
        name: '',
        address: s.address,
        addressLabel: s.shortAddress,
        value: s.revenue,
        wei: s.revenueWei,
        incomeNode: true,
      })),
      { name: 'Hardware Cost', value: totalActualHardwareCost, usd: true },
      { name: 'L1 Data Cost', value: totalL1Cost, usd: true },
      ...(l1ProveCost > 0
        ? [{ name: 'L1 Prove Cost', value: totalActualProveCost, usd: true }]
        : []),
      ...(l1VerifyCost > 0
        ? [{ name: 'L1 Verify Cost', value: totalActualVerifyCost, usd: true }]
        : []),
      ...seqData.map((s) => ({
        name: 'Profit',
        address: s.address,
        addressLabel: s.shortAddress,
        value: s.profit,
        wei: s.profitWei,
        profitNode: true,
      })),
      { name: 'Taiko DAO', value: baseFeeDaoUsd, wei: (baseFee ?? 0) * 0.25 },
    ];

    const proveIndex =
      l1ProveCost > 0 ? l1Index + 1 : -1;
    const verifyIndex =
      l1VerifyCost > 0 ? l1Index + 1 + (l1ProveCost > 0 ? 1 : 0) : -1;
    profitStartIndex += (l1ProveCost > 0 ? 1 : 0) + (l1VerifyCost > 0 ? 1 : 0);
    daoIndex += (l1ProveCost > 0 ? 1 : 0) + (l1VerifyCost > 0 ? 1 : 0);

    links = [
      ...seqData.map((s, i) => ({
        source: priorityIndex,
        target: baseIndex + i,
        value: s.priorityUsd,
      })),
      ...seqData.map((s, i) => ({
        source: baseFeeIndex,
        target: baseIndex + i,
        value: s.baseUsd,
      })),
      { source: baseFeeIndex, target: daoIndex, value: baseFeeDaoUsd },
      ...seqData.map((s, i) => ({
        source: baseIndex + i,
        target: hardwareIndex,
        value: s.actualHardwareCost,
      })),
      ...seqData.map((s, i) => ({
        source: baseIndex + i,
        target: l1Index,
        value: s.actualL1Cost,
      })),
      ...seqData.map((s, i) => ({
        source: subsidyStartIndex + i,
        target: l1Index,
        value: s.subsidyUsd,
      })),
      ...seqData.map((s, i) => ({
        source: baseIndex + i,
        target: profitStartIndex + i,
        value: s.profit,
      })),
    ].filter((l) => l.value > 0);

    if (l1ProveCost > 0 && proveIndex >= 0) {
      links.push(
        ...seqData.map((s, i) => ({
          source: baseIndex + i,
          target: proveIndex,
          value: s.actualProveCost,
        })),
      );
    }
    if (l1VerifyCost > 0 && verifyIndex >= 0) {
      links.push(
        ...seqData.map((s, i) => ({
          source: baseIndex + i,
          target: verifyIndex,
          value: s.actualVerifyCost,
        })),
      );
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

  const formatTooltipValue = (value: number, itemData?: any) => {
    const usd = formatUsd(value);
    if (itemData?.wei != null) {
      return `${formatEth(itemData.wei)} (${usd})`;
    }
    if (!itemData?.usd && ethPrice) {
      const wei = (value / ethPrice) * WEI_TO_ETH;
      return `${formatEth(wei)} (${usd})`;
    }
    return usd;
  };

  const tooltipContent = ({ active, payload }: TooltipProps<number, string>) => {
    if (!active || !payload?.[0]) return null;

    const { value = 0, payload: itemData } = payload![0];

    if (itemData.source != null && itemData.target != null) {
      const sourceNode = data.nodes[itemData.source] as any;
      const targetNode = data.nodes[itemData.target] as any;
      const formatLabel = (node: any) => {
        if (node.profitNode && node.addressLabel) {
          return `${node.addressLabel} Profit`;
        }
        if (node.incomeNode && node.addressLabel) {
          return `${node.addressLabel} Income`;
        }
        return node.addressLabel ?? node.address ?? node.name;
      };
      const sourceLabel = formatLabel(sourceNode);
      const targetLabel = formatLabel(targetNode);

      return (
        <div className="bg-white dark:bg-gray-800 p-2 border border-gray-200 dark:border-gray-700 rounded shadow-sm">
          <p className="text-sm font-medium">
            {sourceLabel} → {targetLabel}
          </p>
          <p className="text-sm text-gray-600 dark:text-gray-300">
            {formatTooltipValue(value, itemData)}
          </p>
        </div>
      );
    }

    const nodeLabel = (() => {
      if (itemData.profitNode && itemData.addressLabel) {
        return `${itemData.addressLabel} Profit`;
      }
      if (itemData.incomeNode && itemData.addressLabel) {
        return `${itemData.addressLabel} Income`;
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
    <div className="mt-6" style={{ height: 240 }}>
      <ResponsiveContainer width="100%" height="100%">
        <Sankey
          data={data}
          node={createSankeyNode(textColor)}
          nodePadding={10}
          nodeWidth={10}
          margin={{ top: 10, right: 120, bottom: 10, left: 10 }}
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
