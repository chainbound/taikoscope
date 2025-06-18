import React from 'react';
import { ResponsiveContainer, Sankey, Tooltip } from 'recharts';
import useSWR from 'swr';
import { fetchDashboardData } from '../services/apiService';
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

  return (
    <g>
      <rect
        x={x}
        y={y}
        width={width}
        height={height}
        fill="#10b981"
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

export const FeeFlowChart: React.FC<FeeFlowChartProps> = ({
  timeRange,
  cloudCost,
  proverCost,
  address,
}) => {
  const { data: dashRes } = useSWR(['dashboardData', timeRange, address], () =>
    fetchDashboardData(timeRange, address),
  );
  const { data: ethPrice = 0 } = useEthPrice();

  const priorityFee = dashRes?.data?.priority_fee ?? null;
  const baseFee = dashRes?.data?.base_fee ?? null;

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
  const baseFeeSeqUsd = baseFeeUsd * 0.75;
  const baseFeeDaoUsd = baseFeeUsd * 0.25;

  // Scale operational costs to the selected time range
  const hours = rangeToHours(timeRange);
  const cloudCostScaled = (cloudCost / MONTH_HOURS) * hours;
  const proverCostScaled = (proverCost / MONTH_HOURS) * hours;

  // Calculate sequencer profit
  const totalRevenue = priorityFeeUsd + baseFeeSeqUsd;
  const totalCosts = cloudCostScaled + proverCostScaled;
  const sequencerProfit = Math.max(0, totalRevenue - totalCosts);

  // Build Sankey data
  const data = {
    nodes: [
      { name: 'Priority Fee', value: priorityFeeUsd },
      { name: 'Base Fee', value: baseFeeUsd },
      { name: 'Sequencers', value: totalRevenue },
      { name: 'Cloud Cost', value: cloudCostScaled },
      { name: 'Prover Cost', value: proverCostScaled },
      { name: 'Profit', value: sequencerProfit },
      { name: 'Taiko DAO', value: baseFeeDaoUsd },
    ],
    links: [
      {
        source: 0,
        target: 2,
        value: priorityFeeUsd,
      },
      {
        source: 1,
        target: 2,
        value: baseFeeSeqUsd,
      },
      {
        source: 1,
        target: 6,
        value: baseFeeDaoUsd,
      },
      {
        source: 2,
        target: 3,
        value: cloudCostScaled,
      },
      {
        source: 2,
        target: 4,
        value: proverCostScaled,
      },
      {
        source: 2,
        target: 5,
        value: sequencerProfit,
      },
    ].filter((link) => link.value > 0), // Only show links with positive values
  };

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
          link={{ stroke: '#94a3b8', strokeOpacity: 0.2 }}
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
