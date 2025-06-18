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

  const priority = dashRes?.data?.priority_fee ?? null;
  const base = dashRes?.data?.base_fee ?? null;
  if (priority == null && base == null) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }

  const priorityUsd = ((priority ?? 0) / 1e18) * ethPrice;
  const baseUsd = ((base ?? 0) / 1e18) * ethPrice;
  const hours = rangeToHours(timeRange);
  const MONTH_HOURS = 30 * 24;
  const cloudCostScaled = (cloudCost / MONTH_HOURS) * hours;
  const proverCostScaled = (proverCost / MONTH_HOURS) * hours;

  // Calculate sequencer profits (total fees minus operational costs)
  const totalRevenue = priorityUsd + baseUsd;
  const sequencerProfit = Math.max(0, totalRevenue - cloudCostScaled - proverCostScaled);

  const data = {
    nodes: [
      { name: 'Priority Fee' },
      { name: 'Base Fee' },
      { name: 'Sequencers' },
      { name: 'Cloud Cost' },
      { name: 'Prover Cost' },
      { name: 'Sequencer Profit' },
    ],
    links: [
      { source: 0, target: 2, value: priorityUsd, name: 'Priority Fee to Sequencers' },
      { source: 1, target: 2, value: baseUsd, name: 'Base Fee to Sequencers' },
      { source: 2, target: 3, value: cloudCostScaled, name: 'Cloud Costs' },
      { source: 2, target: 4, value: proverCostScaled, name: 'Prover Costs' },
      { source: 2, target: 5, value: sequencerProfit, name: 'Sequencer Profit' },
    ],
  };

  const renderLink = (props: any) => {
    const {
      sourceX,
      targetX,
      sourceY,
      targetY,
      sourceControlX,
      targetControlX,
      linkWidth,
    } = props;
    const path = `M${sourceX},${sourceY} C${sourceControlX},${sourceY} ${targetControlX},${targetY} ${targetX},${targetY}`;
    return (
      <path
        d={path}
        fill="none"
        stroke="#333"
        strokeWidth={linkWidth}
        strokeOpacity="0.2"
      />
    );
  };

  const renderNode = (props: any) => {
    const { x, y, width, height, index } = props;
    const node = data.nodes[index];
    return (
      <g>
        <rect
          x={x}
          y={y}
          width={width}
          height={height}
          fill="#4ade80"
          stroke="#333"
          strokeWidth={1}
        />
        <text
          x={x + width / 2}
          y={y + height / 2}
          textAnchor="middle"
          dominantBaseline="middle"
          fontSize={12}
          fill="#333"
          fontWeight="500"
        >
          {node.name}
        </text>
      </g>
    );
  };

  return (
    <div className="mt-6" style={{ height: 240 }}>
      <ResponsiveContainer width="100%" height="100%">
        <Sankey
          data={data}
          nodePadding={10}
          node={renderNode}
          link={renderLink}
          sort={true}
        >
          <Tooltip
            formatter={(v: number) => `$${v.toFixed(2)}`}
            labelFormatter={() => ''}
          />
        </Sankey>
      </ResponsiveContainer>
    </div>
  );
};
