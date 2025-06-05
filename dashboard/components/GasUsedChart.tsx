import React, { useState, useEffect } from 'react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Brush,
} from 'recharts';
import { TimeSeriesData } from '../types';
import { formatLargeNumber, formatTime } from '../utils';

interface GasUsedChartProps {
  data: TimeSeriesData[];
  lineColor: string;
}

const GasUsedChartComponent: React.FC<GasUsedChartProps> = ({
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
  const [brushRange, setBrushRange] = useState({
    startIndex: 0,
    endIndex: data.length - 1,
  });

  useEffect(() => {
    setBrushRange({
      startIndex: 0,
      endIndex: data.length - 1,
    });
  }, [data]);

  const clampedRange = React.useMemo(
    () => ({
      startIndex: Math.max(0, Math.min(brushRange.startIndex, data.length - 1)),
      endIndex: Math.max(0, Math.min(brushRange.endIndex, data.length - 1)),
    }),
    [brushRange, data.length],
  );

  const handleBrushChange = (range: {
    startIndex?: number;
    endIndex?: number;
  }) => {
    if (
      range.startIndex == null ||
      range.endIndex == null ||
      !Number.isFinite(range.startIndex) ||
      !Number.isFinite(range.endIndex)
    )
      return;
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
        margin={{ top: 5, right: 70, left: 80, bottom: 40 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
        <XAxis
          dataKey="value"
          tickFormatter={(v: number) => v.toLocaleString()}
          stroke="#666666"
          fontSize={12}
          label={{
            value: 'Block Number',
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
          domain={['auto', 'auto']}
          tickFormatter={(v: number) => formatLargeNumber(v)}
          label={{
            value: 'Gas Used',
            angle: -90,
            position: 'insideLeft',
            offset: -16,
            fontSize: 10,
            fill: '#666666',
          }}
        />
        <Tooltip
          labelFormatter={(label: number) => `Block ${label.toLocaleString()}`}
          formatter={(value: number) => [formatLargeNumber(value), 'gas']}
          contentStyle={{
            backgroundColor: 'rgba(255, 255, 255, 0.8)',
            borderColor: lineColor,
          }}
          labelStyle={{ color: '#333' }}
        />
        <Line
          type="monotone"
          dataKey="timestamp"
          stroke={lineColor}
          strokeWidth={2}
          dot={false}
          activeDot={data.length <= 100 ? { r: 6 } : false}
          name="Gas Used"
        />
        <Brush
          dataKey="timestamp"
          height={20}
          stroke={lineColor}
          padding={{ left: 40, right: 40 }}
          startIndex={clampedRange.startIndex}
          endIndex={clampedRange.endIndex}
          onChange={handleBrushChange}
          tickFormatter={(v: number) => new Date(v).toLocaleString()}
        />
      </LineChart>
    </ResponsiveContainer>
  );
};

export const GasUsedChart = React.memo(GasUsedChartComponent);
