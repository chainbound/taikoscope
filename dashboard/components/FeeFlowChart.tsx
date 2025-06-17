import React from 'react';
import { ResponsiveContainer, Sankey, Tooltip } from 'recharts';
import useSWR from 'swr';
import { fetchDashboardData } from '../services/apiService';
import { useEthPrice } from '../services/priceService';
import { TimeRange } from '../types';

interface FeeFlowChartProps {
  timeRange: TimeRange;
  address?: string;
}

export const FeeFlowChart: React.FC<FeeFlowChartProps> = ({
  timeRange,
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

  const data = {
    nodes: [{ name: 'Users' }, { name: 'Sequencers' }, { name: 'Taiko DAO' }],
    links: [
      { source: 0, target: 1, value: priorityUsd },
      { source: 0, target: 2, value: baseUsd },
    ],
  };

  return (
    <div className="mt-6" style={{ height: 240 }}>
      <ResponsiveContainer width="100%" height="100%">
        <Sankey
          data={data}
          nodePadding={10}
          node={{ stroke: '#888' }}
          link={{ stroke: '#ccc' }}
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
