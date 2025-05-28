import React from 'react';
import {
  PieChart,
  Pie,
  Cell,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from 'recharts';
import type { Payload } from 'recharts/types/component/DefaultTooltipContent';
import { PieChartDataItem } from '../types';
import { formatSequencerTooltip } from '../utils';

interface SequencerPieChartProps {
  data: PieChartDataItem[];
}

// Updated colors as per request
const SEQUENCER_COLORS: { [key: string]: string } = {
  Nethermind: '#0288d1',
  Titan: '#00d992',
  Chainbound: '#ffc837',
};

const TAIKO_PINK = '#e81899'; // Updated Taiko Pink
const FALLBACK_COLORS = [
  TAIKO_PINK,
  '#E573B5',
  '#5DA5DA',
  '#FAA43A',
  '#60BD68',
  '#F17CB0',
  '#B2912F',
  '#B276B2',
  '#DECF3F',
  '#F15854',
];

export const SequencerPieChart: React.FC<SequencerPieChartProps> = ({
  data,
}) => {
  if (!data || data.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }

  return (
    <ResponsiveContainer width="100%" height="100%">
      <PieChart margin={{ top: 5, right: 5, bottom: 5, left: 5 }}>
        <Pie
          data={data}
          cx="50%"
          cy="50%"
          outerRadius="80%"
          fill="#8884d8"
          dataKey="value"
          nameKey="name"
        >
          {data.map((entry, index) => {
            const color =
              SEQUENCER_COLORS[entry.name] ||
              FALLBACK_COLORS[index % FALLBACK_COLORS.length];
            return <Cell key={`cell-${index}`} fill={color} />;
          })}
        </Pie>
        <Tooltip
          formatter={(
            _,
            name: string,
            item: Payload<number, string>,
          ) => {
            const payload = item.payload as PieChartDataItem;
            return [formatSequencerTooltip(data, payload.value), name];
          }}
        />
        <Legend verticalAlign="bottom" wrapperStyle={{ paddingTop: '10px' }} />
      </PieChart>
    </ResponsiveContainer>
  );
};
