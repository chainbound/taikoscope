import React from 'react';
import { ResponsiveContainer, Sankey, Tooltip } from 'recharts';
import { TAIKO_PINK } from '../theme';
import useSWR from 'swr';
import { fetchL2Fees } from '../services/apiService';
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

// Simple node component that renders label with USD value
const SankeyNode = ({ x, y, width, height, payload }: any) => {
  const nodeValue = payload?.value;
  const formattedValue = nodeValue != null ? formatUsd(nodeValue) : '';
  const isCostNode =
    payload.name === 'Cloud Cost' || payload.name === 'Prover Cost';

  return (
    <g>
      <rect
        x={x}
        y={y}
        width={width}
        height={height}
        fill={isCostNode ? '#ef4444' : TAIKO_PINK}
        fillOpacity={0.8}
      />
      <text
        x={x + width + 6}
        y={y + height / 2}
        textAnchor="start"
        dominantBaseline="middle"
        fontSize={12}
        fill="#374151"
      >
        {payload.name}
        {formattedValue && (
          <tspan fill="#6b7280" fontSize={11}>
            {' '}
            ({formattedValue})
          </tspan>
        )}
      </text>
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
  ...rest
}: any) => {
  const isCost =
    payload.target.name === 'Cloud Cost' ||
    payload.target.name === 'Prover Cost';

  return (
    <path
      className="recharts-sankey-link"
      d={`M${sourceX},${sourceY}C${sourceControlX},${sourceY} ${targetControlX},${targetY} ${targetX},${targetY}`}
      fill="none"
      stroke={isCost ? '#ef4444' : '#94a3b8'}
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
  address,
}) => {
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
  const baseFeeDaoUsd = baseFeeUsd * 0.25;
  const seqCount = sequencerFees.length || 1;

  // Scale operational costs to the selected time range
  const hours = rangeToHours(timeRange);
  const cloudCostPerSeq = (cloudCost / MONTH_HOURS) * hours;
  const proverCostPerSeq = (proverCost / MONTH_HOURS) * hours;
  const cloudCostScaled = cloudCostPerSeq * seqCount;
  const proverCostScaled = proverCostPerSeq * seqCount;

  const seqData = sequencerFees.map((f) => {
    const priorityUsd = ((f.priority_fee ?? 0) / WEI_TO_ETH) * ethPrice;
    const baseUsd = ((f.base_fee ?? 0) / WEI_TO_ETH) * ethPrice * 0.75;
    const revenue = priorityUsd + baseUsd;
    const profit = Math.max(0, revenue - cloudCostPerSeq - proverCostPerSeq);
    return { address: f.address, priorityUsd, baseUsd, revenue, profit };
  });

  const totalProfit = seqData.reduce((acc, s) => acc + s.profit, 0);

  // Build Sankey data with one node per sequencer
  const baseIndex = 2; // first sequencer node index
  const cloudIndex = baseIndex + seqData.length;
  const proverIndex = cloudIndex + 1;
  const profitIndex = proverIndex + 1;
  const daoIndex = profitIndex + 1;

  const nodes = [
    { name: 'Priority Fee', value: priorityFeeUsd },
    { name: 'Base Fee', value: baseFeeUsd },
    ...seqData.map((s) => ({ name: s.address, value: s.revenue })),
    { name: 'Cloud Cost', value: cloudCostScaled },
    { name: 'Prover Cost', value: proverCostScaled },
    { name: 'Profit', value: totalProfit },
    { name: 'Taiko DAO', value: baseFeeDaoUsd },
  ];

  const links = [
    ...seqData.map((s, i) => ({ source: 0, target: baseIndex + i, value: s.priorityUsd })),
    ...seqData.map((s, i) => ({ source: 1, target: baseIndex + i, value: s.baseUsd })),
    { source: 1, target: daoIndex, value: baseFeeDaoUsd },
    ...seqData.map((_, i) => ({ source: baseIndex + i, target: cloudIndex, value: cloudCostPerSeq })),
    ...seqData.map((_, i) => ({ source: baseIndex + i, target: proverIndex, value: proverCostPerSeq })),
    ...seqData.map((s, i) => ({ source: baseIndex + i, target: profitIndex, value: s.profit })),
  ].filter((l) => l.value > 0);

  const data = { nodes, links };

  const formatTooltipValue = (value: number) => formatUsd(value);

  const tooltipContent = ({ active, payload }: any) => {
    if (!active || !payload?.[0]) return null;

    const { value, payload: linkData } = payload[0];
    const sourceNode = data.nodes[linkData.source];
    const targetNode = data.nodes[linkData.target];

    return (
      <div className="bg-white p-2 border border-gray-200 rounded shadow-sm">
        <p className="text-sm font-medium">
          {sourceNode.name} → {targetNode.name}
        </p>
        <p className="text-sm text-gray-600">{formatTooltipValue(value)}</p>
      </div>
    );
  };

  return (
    <div className="mt-6" style={{ height: 240 }}>
      <ResponsiveContainer width="100%" height="100%">
        <Sankey
          data={data}
          node={SankeyNode}
          nodePadding={10}
          nodeWidth={10}
          margin={{ top: 10, right: 120, bottom: 10, left: 10 }}
          sort={false}
          iterations={32}
          link={SankeyLink}
        >
          <Tooltip
            content={tooltipContent}
            contentStyle={{
              backgroundColor: 'white',
              border: '1px solid #e5e7eb',
              borderRadius: '0.375rem',
            }}
          />
        </Sankey>
      </ResponsiveContainer>
    </div>
  );
};
