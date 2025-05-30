import React, { useState } from 'react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  Brush,
} from 'recharts';
import { TimeSeriesData } from '../types';
import { formatDecimal, formatInterval, shouldShowMinutes } from '../utils';

interface BlockTimeChartProps {
  data: TimeSeriesData[];
  lineColor: string;
}

export const BlockTimeChart: React.FC<BlockTimeChartProps> = ({
  data,
  lineColor,
}) => {
  if (!data || data.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }
  const showMinutes = shouldShowMinutes(data);
  const [brushRange, setBrushRange] = useState({
    startIndex: Math.max(0, data.length - 50),
    endIndex: data.length - 1,
  });

  const handleBrushChange = (range: {
    startIndex?: number;
    endIndex?: number;
  }) => {
    if (range.startIndex == null || range.endIndex == null) return;
    const maxRange = 500;
    if (range.endIndex - range.startIndex > maxRange) {
      setBrushRange({
        startIndex: range.endIndex - maxRange,
        endIndex: range.endIndex,
      });
    } else {
      setBrushRange({ startIndex: range.startIndex, endIndex: range.endIndex });
    }
  };
  return (
    <ResponsiveContainer width="100%" height="100%">
      <LineChart
        data={data}
        margin={{ top: 5, right: 30, left: 20, bottom: 50 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
        <XAxis
          dataKey="value"
          tickFormatter={(v: number) => v.toLocaleString()}
          stroke="#666666"
          fontSize={12}
          label={{
            value: 'Block Number',
            position: 'bottom',
            offset: 0,
            fontSize: 10,
            fill: '#666666',
          }}
          padding={{ left: 10, right: 10 }}
        />
        <YAxis
          stroke="#666666"
          fontSize={12}
          domain={['auto', 'auto']}
          tickFormatter={(v) =>
            showMinutes
              ? String(Number(formatDecimal(v / 60000)))
              : String(Number(formatDecimal(v / 1000)))
          }
          label={{
            value: showMinutes ? 'Minutes' : 'Seconds',
            angle: -90,
            position: 'insideLeft',
            offset: -16,
            fontSize: 10,
            fill: '#666666',
          }}
        />
        <Tooltip
          labelFormatter={(label: number) => `Block ${label.toLocaleString()}`}
          formatter={(value: number) => [formatInterval(value, showMinutes)]}
          contentStyle={{
            backgroundColor: 'rgba(255, 255, 255, 0.8)',
            borderColor: lineColor,
          }}
          labelStyle={{ color: '#333' }}
        />
        <Legend
          verticalAlign="bottom"
          align="right"
          wrapperStyle={{ right: 20, bottom: 0 }}
        />
        <Line
          type="monotone"
          dataKey="timestamp"
          stroke={lineColor}
          strokeWidth={2}
          dot={false}
          activeDot={data.length <= 100 ? { r: 6 } : false}
          name="Time"
        />
        <Brush
          dataKey="value"
          height={20}
          stroke={lineColor}
          startIndex={brushRange.startIndex}
          endIndex={brushRange.endIndex}
          onChange={handleBrushChange}
        />
      </LineChart>
    </ResponsiveContainer>
  );
};
