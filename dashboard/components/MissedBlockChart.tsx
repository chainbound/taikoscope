import React, { useState, useEffect, useMemo } from 'react';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Brush,
} from 'recharts';
import type { MissedBlockProposal } from '../types';
import { TAIKO_PINK } from '../theme';

interface MissedBlockChartProps {
  data: MissedBlockProposal[];
}

export const MissedBlockChart: React.FC<MissedBlockChartProps> = ({ data }) => {
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

  const [brushRange, setBrushRange] = useState({
    startIndex: 0,
    endIndex: sortedData.length - 1,
  });

  useEffect(() => {
    setBrushRange({ startIndex: 0, endIndex: sortedData.length - 1 });
  }, [sortedData]);

  const handleBrushChange = (range: { startIndex?: number; endIndex?: number }) => {
    if (range.startIndex == null || range.endIndex == null) return;
    const maxRange = 500;
    if (range.endIndex - range.startIndex > maxRange) {
      setBrushRange({ startIndex: range.endIndex - maxRange, endIndex: range.endIndex });
    } else {
      setBrushRange({ startIndex: range.startIndex, endIndex: range.endIndex });
    }
  };

  return (
    <ResponsiveContainer width="100%" height="100%">
      <BarChart data={sortedData} margin={{ top: 5, right: 70, left: 20, bottom: 40 }}>
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
          contentStyle={{ backgroundColor: 'rgba(255, 255, 255, 0.8)', borderColor: TAIKO_PINK }}
          labelStyle={{ color: '#333' }}
        />
        <Bar dataKey={() => 1} fill={TAIKO_PINK} name="Missed" />
        <Brush
          dataKey="slot"
          height={20}
          stroke={TAIKO_PINK}
          startIndex={brushRange.startIndex}
          endIndex={brushRange.endIndex}
          onChange={handleBrushChange}
        />
      </BarChart>
    </ResponsiveContainer>
  );
};
