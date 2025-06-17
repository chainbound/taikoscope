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

  const data = {
    nodes: [
      { name: 'Users' },
      { name: 'Sequencers' },
      { name: 'Taiko DAO' },
      { name: 'Cloud Providers' },
      { name: 'Provers' },
    ],
    links: [
      { source: 0, target: 1, value: priorityUsd, name: 'Priority Fee' },
      { source: 0, target: 2, value: baseUsd, name: 'Base Fee' },
      { source: 1, target: 3, value: cloudCostScaled, name: 'Cloud Cost' },
      { source: 1, target: 4, value: proverCostScaled, name: 'Prover Cost' },
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
      index,
    } = props;
    const { name, value } = data.links[index];
    const path = `M${sourceX},${sourceY} C${sourceControlX},${sourceY} ${targetControlX},${targetY} ${targetX},${targetY}`;
    const midX = (sourceX + targetX) / 2;
    const midY = (sourceY + targetY) / 2;
    return (
      <g>
        <path
          d={path}
          fill="none"
          stroke="#333"
          strokeWidth={linkWidth}
          strokeOpacity="0.2"
        />
        <text
          x={midX}
          y={midY - 4}
          textAnchor="middle"
          fontSize={10}
          fill="#333"
          pointerEvents="none"
        >
          {`${name}: $${value.toFixed(2)}`}
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
          node={{ stroke: '#888' }}
          link={renderLink}
          sort={false}
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
