import React from 'react';
import { ResponsiveContainer, Sankey, Tooltip } from 'recharts';
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

export const FeeFlowChart: React.FC<FeeFlowChartProps> = ({
  timeRange,
  cloudCost,
  proverCost,
  address,
}) => {
  const { data: feeRes } = useSWR(['l2Fees', timeRange, address], () =>
    fetchL2Fees(timeRange, address),
  );
  const { data: ethPrice = 0 } = useEthPrice();

  const priority = feeRes?.data?.priority_fee ?? null;
  const base = feeRes?.data?.base_fee ?? null;
  const l1DataCost = feeRes?.data?.l1_data_cost ?? null;
  if (priority == null && base == null && l1DataCost == null) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }

  const priorityUsd = ((priority ?? 0) / 1e18) * ethPrice;
  const baseUsd = ((base ?? 0) / 1e18) * ethPrice;
  const l1DataCostUsd = ((l1DataCost ?? 0) / 1e18) * ethPrice;
  const hours = rangeToHours(timeRange);
  const MONTH_HOURS = 30 * 24;
  const cloudCostScaled = (cloudCost / MONTH_HOURS) * hours;
  const proverCostScaled = (proverCost / MONTH_HOURS) * hours;
  const profitUsd = Math.max(
    priorityUsd - cloudCostScaled - proverCostScaled - l1DataCostUsd,
    0,
  );

  const data = {
    nodes: [
      { name: 'Users' },
      { name: 'Sequencers' },
      { name: 'Taiko DAO' },
      { name: 'Cloud Providers' },
      { name: 'Provers' },
      { name: 'L1 Data' },
      { name: 'Sequencer Profits' },
    ],
    links: [
      { source: 0, target: 1, value: priorityUsd, name: 'Priority Fee' },
      { source: 0, target: 2, value: baseUsd, name: 'Base Fee' },
      { source: 1, target: 3, value: cloudCostScaled, name: 'Cloud Cost' },
      { source: 1, target: 4, value: proverCostScaled, name: 'Prover Cost' },
      { source: 1, target: 5, value: l1DataCostUsd, name: 'L1 Data Cost' },
      { source: 1, target: 6, value: profitUsd, name: 'Sequencer Profit' },
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
