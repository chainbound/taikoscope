import React from 'react';
import { ResponsiveContainer, Sankey, Tooltip } from 'recharts';
import type { TooltipProps } from 'recharts';
import { formatEth } from '../utils';
import { lightTheme } from '../theme';
import { useTheme } from '../contexts/ThemeContext';
import {
  convertFeesToUsd,
  processSequencerData,
  generateFallbackSankeyData,
  generateMultiSequencerSankeyData,
  validateChartData,
  safeValue,
  type SankeyChartData,
} from '../utils/feeFlowCalculations';

const NODE_GREEN = '#22c55e';
const GWEI_TO_ETH = 1e9;
import useSWR from 'swr';
import { fetchL2FeesComponents, type L2FeesComponentsResponse } from '../services/apiService';
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
  /** Pre-fetched L2 fees + components data to avoid duplicate requests */
  feesData?: L2FeesComponentsResponse | null;
}

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
    const LINE_HEIGHT = 12; // More conservative estimate for 12px font
    const NUM_LINES = 2; // name + value
    const blockHalf = (LINE_HEIGHT * (NUM_LINES - 1)) / 2; // = 6 px

    const isCostNode =
      payload.name === 'Hardware Cost' ||
      payload.name === 'Proposing Cost' ||
      payload.name === 'Proving Cost';
    const isSubsidyNode =
      payload.name === 'Subsidy' ||
      (typeof payload.name === 'string' && payload.name.includes('Subsidy'));
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
          fill={
            isCostNode
              ? '#ef4444'
              : isPinkNode
                ? 'var(--color-brand)'
                : isSubsidyNode
                  ? NODE_GREEN
                  : NODE_GREEN
          }
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
  feesData,
}) => {
  const { theme } = useTheme();
  const textColor = theme === 'dark' ? '#ffffff' : lightTheme.foreground;
  const { data: ethPrice = 0 } = useEthPrice();
  // Fallback fetch when feesData not provided via props
  const { data: feesRes } = useSWR(
    feesData === undefined ? ['l2FeesComponents', timeRange] : null,
    () => fetchL2FeesComponents(timeRange),
  );
  const effectiveFees = feesData ?? feesRes?.data ?? null;

  const priorityFee = effectiveFees?.priority_fee ?? null;
  const baseFee = effectiveFees?.base_fee ?? null;
  const allSequencerFees = effectiveFees?.sequencers ?? [];
  const sequencerFees = address
    ? allSequencerFees.filter(
        (s) => s.address.toLowerCase() === address.toLowerCase(),
      )
    : allSequencerFees;

  // Memoized tooltip value formatter to avoid unnecessary re-renders
  // NOTE: Depends on `ethPrice`, so it is recreated only when the price changes
  const formatTooltipValue = React.useCallback(
    (value: number, itemData?: any) => {
      const usd = formatUsd(value);

      // If the item already has a `gwei` value, use it directly
      if (itemData?.gwei != null) {
        return `${formatEth(itemData.gwei, 4)} (${usd})`;
      }

      // Otherwise, attempt to derive `gwei` from USD using the current ETH price
      if (ethPrice && ethPrice > 0) {
        const gwei = (value / ethPrice) * GWEI_TO_ETH;
        return `${formatEth(gwei, 4)} (${usd})`;
      }

      // Fallback when ETH price is unavailable: return USD only
      return usd;
    },
    [ethPrice],
  );

  // Node value formatter - shows only ETH values without USD
  const formatNodeValue = React.useCallback(
    (value: number, itemData?: any) => {
      // If the item already has a `gwei` value, use it directly
      if (itemData?.gwei != null) {
        return formatEth(itemData.gwei, 4);
      }

      // Otherwise, attempt to derive `gwei` from USD using the current ETH price
      if (ethPrice && ethPrice > 0) {
        const gwei = (value / ethPrice) * GWEI_TO_ETH;
        return formatEth(gwei, 4);
      }

      // Fallback when ETH price is unavailable: return 0 ETH
      return '0 ETH';
    },
    [ethPrice],
  );

  const NodeComponent = React.useMemo(
    () => createSankeyNode(textColor, formatNodeValue),
    [textColor, formatNodeValue],
  );

  if (!effectiveFees) {
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

  // Do not block rendering if ETH price is missing; downstream formatters
  // already fallback to USD-only values and 0 ETH when needed.

  // Convert fees to USD using utility function
  const {
    priorityFeeUsd,
    baseFeeUsd,
    l1DataCostTotalUsd,
    l1ProveCost,
    baseFeeDaoUsd,
  } = convertFeesToUsd({
    priorityFee: priorityFee ?? 0,
    baseFee: baseFee ?? 0,
    l1DataCost: effectiveFees?.l1_data_cost ?? 0,
    proveCost: effectiveFees?.prove_cost ?? 0,
    ethPrice,
  });

  // Scale operational costs to the selected time range
  const hours = rangeToHours(timeRange);
  const sequencerCount = Math.max(1, totalSequencers ?? sequencerFees.length);
  const {
    totalUsd: rawTotalHardwareCost,
    perSequencerUsd: rawHardwareCostPerSeq,
  } = calculateHardwareCost(cloudCost, proverCost, sequencerCount, hours);
  const totalHardwareCost = safeValue(rawTotalHardwareCost);
  const hardwareCostPerSeq = safeValue(rawHardwareCostPerSeq);

  // Process sequencer data using utility function
  const seqData = processSequencerData(sequencerFees, ethPrice, hardwareCostPerSeq);

  // Generate Sankey chart data
  let chartData: SankeyChartData;

  if (seqData.length === 0) {
    // Fallback: create a single "Sequencers" node to route fees through
    chartData = generateFallbackSankeyData({
      priorityFeeUsd,
      baseFeeUsd,
      baseFeeDaoUsd,
      l1DataCostTotalUsd,
      l1ProveCost,
      totalHardwareCost,
      priorityFee,
      baseFee,
      ethPrice,
    });
  } else {
    // Multi-sequencer scenario
    chartData = generateMultiSequencerSankeyData(
      seqData,
      priorityFeeUsd,
      baseFeeUsd,
      baseFeeDaoUsd,
      priorityFee,
      baseFee,
    );
  }

  // Validate and process chart data using utility function
  const validatedChartData = validateChartData(chartData);

  // Check if we have valid data after processing
  if (
    !validatedChartData.nodes ||
    !validatedChartData.links ||
    validatedChartData.nodes.length === 0 ||
    validatedChartData.links.length === 0
  ) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        Unable to create flow chart with current data
      </div>
    );
  }

  const data = validatedChartData;

  const tooltipContent = ({
    active,
    payload,
  }: TooltipProps<number, string>) => {
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
      <div
        className="bg-card text-card-fg p-2 border border-gray-100 dark:border-border rounded shadow-sm"
        style={{ backgroundColor: 'var(--card)', opacity: 1 }}
      >
        <p className="text-sm font-medium dark:text-white">{nodeLabel}</p>
        <p className="text-sm text-gray-600 dark:text-white">
          {formatTooltipValue(value, itemData)}
        </p>
      </div>
    );
  };

  return (
    <div className="mt-6 fee-flow-chart" style={{ height }}>
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
            wrapperStyle={{ opacity: 1 }}
            contentStyle={{
              // Force opaque background (no translucency)
              backgroundColor: 'var(--card)',
              border: theme === 'dark' ? '1px solid #334155' : '1px solid #e5e7eb',
              borderRadius: '0.375rem',
              opacity: 1,
            }}
          />
        </Sankey>
      </ResponsiveContainer>
    </div>
  );
};
