import React, { useState, useEffect } from 'react';
import {
  LineChart,
  Line,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Brush,
} from 'recharts';
import { TimeSeriesData } from '../types';
import {
  formatDecimal,
  formatInterval,
  shouldShowMinutes,
  formatTime,
} from '../utils';

interface BlockTimeChartProps {
  data: TimeSeriesData[];
  lineColor: string;
  histogram?: boolean;
}

const BlockTimeChartComponent: React.FC<BlockTimeChartProps> = ({
  data,
  lineColor,
  histogram = false,
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
    startIndex: 0,
    endIndex: data.length - 1,
  });

  useEffect(() => {
    setBrushRange({
      startIndex: 0,
      endIndex: data.length - 1,
    });
  }, [data]);

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
  const ChartComponent = histogram ? BarChart : LineChart;
  return (
    <ResponsiveContainer width="100%" height="100%">
      <ChartComponent
        data={data}
        margin={{ top: 5, right: 90, left: 20, bottom: 40 }}
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
        {histogram ? (
          <Bar dataKey="timestamp" fill={lineColor} name="Time" />
        ) : (
          <Line
            type="monotone"
            dataKey="timestamp"
            stroke={lineColor}
            strokeWidth={2}
            dot={false}
            activeDot={data.length <= 100 ? { r: 6 } : false}
            name="Time"
          />
        )}
        <Brush
          dataKey="timestamp"
          height={20}
          stroke={lineColor}
          startIndex={brushRange.startIndex}
          endIndex={brushRange.endIndex}
          onChange={handleBrushChange}
          tickFormatter={(v: number) => formatTime(v)}
        />
      </ChartComponent>
    </ResponsiveContainer>
  );
};

export const BlockTimeChart = React.memo(BlockTimeChartComponent);
