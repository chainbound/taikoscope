import React, { useMemo } from 'react';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import { useIsMobile } from '../hooks/useIsMobile';
import type { MissedBlockProposal } from '../types';
// brand color via CSS variable

interface MissedBlockChartProps {
  data: MissedBlockProposal[];
}

const MissedBlockChartComponent: React.FC<MissedBlockChartProps> = ({
  data,
}) => {
  const isMobile = useIsMobile();
  const sortedData = useMemo(
    () => (data ? [...data].sort((a, b) => a.slot - b.slot) : []),
    [data],
  );
  if (sortedData.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }

  return (
    <ResponsiveContainer width="100%" height="100%">
      <BarChart
        data={sortedData}
        margin={{ top: 5, right: 20, left: 20, bottom: 40 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="var(--chart-grid)" />
        <XAxis
          dataKey="slot"
          tickFormatter={(v: number) => v.toLocaleString()}
          stroke="var(--chart-tick)"
          fontSize={12}
          label={{
            value: 'Slot',
            position: 'insideBottom',
            offset: -35,
            fontSize: 10,
            fill: 'var(--chart-tick)',
          }}
          padding={{ left: isMobile ? 5 : 10, right: isMobile ? 5 : 10 }}
        />
        <YAxis
          stroke="var(--chart-tick)"
          fontSize={12}
          allowDecimals={false}
          domain={[0, 1]}
          label={{
            value: 'Missed',
            angle: -90,
            position: 'insideLeft',
            offset: -16,
            fontSize: 10,
            fill: 'var(--chart-tick)',
          }}
        />
        <Tooltip
          labelFormatter={(label: number) => `Slot ${label.toLocaleString()}`}
          formatter={() => [1, 'missed']}
          contentStyle={{
            backgroundColor: 'var(--chart-tooltip-bg)',
            borderColor: 'var(--color-brand)',
          }}
          labelStyle={{ color: 'var(--chart-tooltip-label)' }}
        />
        <Bar dataKey={() => 1} fill={'var(--color-brand)'} name="Missed" />
      </BarChart>
    </ResponsiveContainer>
  );
};

export const MissedBlockChart = React.memo(MissedBlockChartComponent);
