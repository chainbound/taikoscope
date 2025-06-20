import React, { useMemo } from 'react';
import { useIsMobile } from '../hooks/useIsMobile';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import type { MissedBlockProposal } from '../types';
import { TAIKO_PINK } from '../theme';

interface MissedBlockChartProps {
  data: MissedBlockProposal[];
}

const MissedBlockChartComponent: React.FC<MissedBlockChartProps> = ({
  data,
}) => {
  if (!data || data.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }

  const sortedData = useMemo(
    () => [...data].sort((a, b) => a.slot - b.slot),
    [data],
  );
  const isMobile = useIsMobile();

  return (
    <ResponsiveContainer width="100%" height="100%">
      <BarChart
        data={sortedData}
        margin={{
          top: 5,
          right: isMobile ? 10 : 20,
          left: isMobile ? 10 : 20,
          bottom: 40,
        }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
        <XAxis
          dataKey="slot"
          tickFormatter={(v: number) => v.toLocaleString()}
          stroke="#666666"
          fontSize={12}
          label={{
            value: 'Slot',
            position: 'insideBottom',
            offset: -35,
            fontSize: 10,
            fill: '#666666',
          }}
          padding={{ left: 10, right: 10 }}
        />
        <YAxis
          stroke="#666666"
          fontSize={12}
          allowDecimals={false}
          domain={[0, 1]}
          label={{
            value: 'Missed',
            angle: -90,
            position: 'insideLeft',
            offset: -16,
            fontSize: 10,
            fill: '#666666',
          }}
        />
        <Tooltip
          labelFormatter={(label: number) => `Slot ${label.toLocaleString()}`}
          formatter={() => [1, 'missed']}
          contentStyle={{
            backgroundColor: 'rgba(255, 255, 255, 0.8)',
            borderColor: TAIKO_PINK,
          }}
          labelStyle={{ color: '#333' }}
        />
        <Bar dataKey={() => 1} fill={TAIKO_PINK} name="Missed" />
      </BarChart>
    </ResponsiveContainer>
  );
};

export const MissedBlockChart = React.memo(MissedBlockChartComponent);
