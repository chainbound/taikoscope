import React from 'react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import { TimeSeriesData } from '../types';
import { formatLargeNumber, formatDateTime } from '../utils';

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
  return (
    <ResponsiveContainer width="100%" height="100%">
      <LineChart
        data={data}
        margin={{ top: 5, right: 20, left: 20, bottom: 40 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
        <XAxis
          dataKey="value"
          tickFormatter={(v: number) => v.toLocaleString()}
          stroke="#666666"
          fontSize={12}
          label={{
            value: 'L2 Block Number',
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
            value: 'Avg Gas Used',
            angle: -90,
            position: 'insideLeft',
            offset: -16,
            fontSize: 10,
            fill: '#666666',
          }}
        />
        <Tooltip
          labelFormatter={(label: number, payload) => {
            const ts = payload?.[0]?.payload?.blockTime;
            const timeStr = ts ? formatDateTime(ts) : '';
            return `Block ${label.toLocaleString()} (${timeStr})`;
          }}
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
          name="Avg Gas Used"
        />
      </LineChart>
    </ResponsiveContainer>
  );
};

export const GasUsedChart = React.memo(GasUsedChartComponent);
