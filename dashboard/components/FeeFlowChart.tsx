import React from 'react';
import { ResponsiveContainer, Sankey, Tooltip } from 'recharts';
import { formatEth } from '../utils';
import { TAIKO_PINK, lightTheme, darkTheme } from '../theme';
import { useTheme } from '../contexts/ThemeContext';

const NODE_GREEN = '#22c55e';
import useSWR from 'swr';
import { fetchL2Fees } from '../services/apiService';
import { getSequencerName } from '../sequencerConfig';
import { useEthPrice } from '../services/priceService';
import { TimeRange } from '../types';
import { rangeToHours } from '../utils/timeRange';

interface FeeFlowChartProps {
  timeRange: TimeRange;
  cloudCost: number;
  proverCost: number;
  l1ProveCost?: number;
  address?: string;
}

const MONTH_HOURS = 30 * 24;
const WEI_TO_ETH = 1e18;

// Format numbers as USD without grouping
const formatUsd = (value: number) => `$${value.toFixed(2)}`;

// Simple node component that renders label with currency-aware value
const createSankeyNode =
  (textColor: string) =>
  ({ x, y, width, height, payload }: any) => {
    const isCostNode =
      payload.name === 'Hardware Cost' ||
      payload.name === 'L1 Data Cost' ||
      payload.name === 'L1 Prove Cost' ||
      payload.name === 'Subsidy' ||
      (typeof payload.name === 'string' && payload.name.includes('Subsidy'));
    const isProfitNode = payload.name === 'Profit' || payload.profitNode;
    const isPinkNode =
      payload.name === 'Taiko DAO' ||
      payload.name === 'Priority Fee' ||
      payload.name === 'Base Fee';
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
          x={x}
          y={y}
          width={width}
          height={height}
          fill={isCostNode ? '#ef4444' : isPinkNode ? TAIKO_PINK : NODE_GREEN}
          fillOpacity={0.8}
        />
        {!hideLabel && (
          <text
            x={x + width + 6}
            y={y + height / 2}
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

const SankeyLink = ({
  sourceX,
  sourceY,
  sourceControlX,
  targetX,
  targetY,
  targetControlX,
  linkWidth,
  payload,
  sourceRelativeY,
  targetRelativeY,
  ...rest
}: any) => {
  const isCost =
    payload.target.name === 'Hardware Cost' ||
    payload.target.name === 'L1 Data Cost' ||
    payload.target.name === 'L1 Prove Cost' ||
    payload.target.name === 'Subsidy' ||
    (typeof payload.target.name === 'string' &&
      payload.target.name.includes('Subsidy'));
  const isProfit =
    payload.target.name === 'Profit' || payload.target.profitNode;

  return (
    <path
      className="recharts-sankey-link"
      d={`M${sourceX},${sourceY}C${sourceControlX},${sourceY} ${targetControlX},${targetY} ${targetX},${targetY}`}
      fill="none"
      stroke={isCost ? '#ef4444' : isProfit ? NODE_GREEN : '#94a3b8'}
      strokeWidth={linkWidth}
      strokeOpacity={0.2}
      {...rest}
    />
  );
};

export const FeeFlowChart: React.FC<FeeFlowChartProps> = ({
  timeRange,
  cloudCost,
  proverCost,
  l1ProveCost = 0,
  address,
}) => {
  const { theme } = useTheme();
  const textColor =
    theme === 'dark' ? darkTheme.foreground : lightTheme.foreground;
  const { data: feeRes } = useSWR(['l2FeesFlow', timeRange, address], () =>
    fetchL2Fees(timeRange, address),
  );
  const { data: ethPrice = 0 } = useEthPrice();

  const priorityFee = feeRes?.data?.priority_fee ?? null;
  const baseFee = feeRes?.data?.base_fee ?? null;
  const sequencerFees = feeRes?.data?.sequencers ?? [];

  if (priorityFee == null && baseFee == null) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }

  // Convert fees to USD
  const priorityFeeUsd = ((priorityFee ?? 0) / WEI_TO_ETH) * ethPrice;
  const baseFeeUsd = ((baseFee ?? 0) / WEI_TO_ETH) * ethPrice;
  const l1DataCostTotalUsd =
    ((feeRes?.data?.l1_data_cost ?? 0) / WEI_TO_ETH) * ethPrice;
  const baseFeeDaoUsd = baseFeeUsd * 0.25;

  // Scale operational costs to the selected time range
  const hours = rangeToHours(timeRange);
  const hardwareCostPerSeq = ((cloudCost + proverCost) / MONTH_HOURS) * hours;
  const totalHardwareCost = hardwareCostPerSeq;

  const seqData = sequencerFees.map((f) => {
    const priorityWei = f.priority_fee ?? 0;
    const baseWei = (f.base_fee ?? 0) * 0.75;
    const l1CostWei = f.l1_data_cost ?? 0;
    const priorityUsd = (priorityWei / WEI_TO_ETH) * ethPrice;
    const baseUsd = (baseWei / WEI_TO_ETH) * ethPrice;
    const l1CostUsd = (l1CostWei / WEI_TO_ETH) * ethPrice;

    const revenue = priorityUsd + baseUsd;
    const revenueWei = priorityWei + baseWei;

    const rawProfit = revenue - hardwareCostPerSeq - l1CostUsd;
    const profit = Math.max(0, rawProfit);
    let remaining = revenue;
    const actualHardwareCost = Math.min(hardwareCostPerSeq, remaining);
    remaining -= actualHardwareCost;
    const actualL1Cost = Math.min(l1CostUsd, remaining);
    remaining -= actualL1Cost;
    const subsidyUsd = l1CostUsd - actualL1Cost;
    const subsidyWei = ethPrice ? (subsidyUsd / ethPrice) * WEI_TO_ETH : 0;
    const profitWei = ethPrice ? (profit / ethPrice) * WEI_TO_ETH : 0;
    const actualHardwareCostWei = ethPrice
      ? (actualHardwareCost / ethPrice) * WEI_TO_ETH
      : 0;
    const actualL1CostWei = ethPrice
      ? (actualL1Cost / ethPrice) * WEI_TO_ETH
      : 0;
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
      l1CostUsd,
      subsidyUsd,
      subsidyWei,
      actualHardwareCostWei,
      actualL1CostWei,
    };
  });

  // Handle case when no sequencer data is available
  let nodes, links;

  if (seqData.length === 0) {
    // Fallback: create a single "Sequencers" node to route fees through
    const sequencerRevenue = priorityFeeUsd + baseFeeUsd * 0.75;
    const maxL1FromRevenue = Math.max(0, sequencerRevenue - totalHardwareCost);
    const actualL1Cost = Math.min(l1DataCostTotalUsd, maxL1FromRevenue);
    const l1Subsidy = l1DataCostTotalUsd - actualL1Cost;
    const sequencerProfit = Math.max(
      0,
      sequencerRevenue - totalHardwareCost - actualL1Cost,
    );
    const sequencerRevenueWei = (priorityFee ?? 0) + (baseFee ?? 0) * 0.75;
    const sequencerProfitWei = ethPrice
      ? (sequencerProfit / ethPrice) * WEI_TO_ETH
      : 0;

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

    if (l1ProveCost > 0) {
      const proveIndex = 6; // insert below L1 Data Cost
      nodes.splice(proveIndex, 0, {
        name: 'L1 Prove Cost',
        value: l1ProveCost,
        usd: true,
      });
    }

    links = [
      { source: 1, target: 3, value: priorityFeeUsd }, // Priority Fee → Sequencers
      { source: 2, target: 3, value: baseFeeUsd * 0.75 }, // 75% Base Fee → Sequencers
      { source: 2, target: 8, value: baseFeeDaoUsd }, // 25% Base Fee → Taiko DAO
      {
        source: 3,
        target: 4,
        value: Math.min(totalHardwareCost, sequencerRevenue),
      }, // Sequencers → Hardware Cost
      {
        source: 3,
        target: 5,
        value: Math.min(
          l1DataCostTotalUsd,
          Math.max(0, sequencerRevenue - totalHardwareCost),
        ),
      }, // Sequencers → L1 Data Cost
      { source: 0, target: 5, value: l1Subsidy }, // Subsidy → L1 Data Cost
      { source: 3, target: 7, value: sequencerProfit }, // Sequencers → Profit
    ].filter((l) => l.value > 0);

    if (l1ProveCost > 0) {
      const proveIndex = 6;
      links.push({ source: 3, target: proveIndex, value: l1ProveCost });
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

    if (l1ProveCost > 0) {
      nodes.splice(l1Index + 1, 0, {
        name: 'L1 Prove Cost',
        value: l1ProveCost,
        usd: true,
      });
      profitStartIndex += 1;
      daoIndex += 1;
    }

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

    if (l1ProveCost > 0) {
      const proveIndex = l1Index + 1;
      links.push(
        ...seqData.map((_, i) => ({
          source: baseIndex + i,
          target: proveIndex,
          value: l1ProveCost / seqData.length,
        })),
      );
    }
  }

  // Remove nodes that have no remaining links after filtering
  const usedIndices = new Set<number>();
  links.forEach((l) => {
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
  const remappedLinks = links.map((l) => ({
    ...l,
    source: indexMap.get(l.source) as number,
    target: indexMap.get(l.target) as number,
  }));

  const data = { nodes: filteredNodes, links: remappedLinks };

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

  const tooltipContent = ({ active, payload }: any) => {
    if (!active || !payload?.[0]) return null;

    const { value, payload: itemData } = payload[0];

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
