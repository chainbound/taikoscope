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
import { useIsMobile } from '../hooks/useIsMobile';
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
  const isMobile = useIsMobile();
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
        <CartesianGrid strokeDasharray="3 3" stroke="var(--chart-grid)" />
        <XAxis
          dataKey="value"
          tickFormatter={(v: number) => v.toLocaleString()}
          stroke="var(--chart-tick)"
          fontSize={12}
          label={{
            value: 'L2 Block Number',
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
          domain={['auto', 'auto']}
          tickFormatter={(v: number) => formatLargeNumber(v)}
          label={{
            value: 'Avg Gas Used',
            angle: -90,
            position: 'insideLeft',
            offset: -16,
            fontSize: 10,
            fill: 'var(--chart-tick)',
          }}
        />
        <Tooltip
          labelFormatter={(label: number, payload) => {
            const ts = payload?.[0]?.payload?.blockTime;
            const timeStr = ts ? formatDateTime(ts) : '';
            return `Block ${label.toLocaleString()} (${timeStr})`;
          }}
          formatter={(value: number) => [formatLargeNumber(value), 'avg gas']}
          contentStyle={{
            backgroundColor: 'var(--chart-tooltip-bg)',
            borderColor: lineColor,
          }}
          labelStyle={{ color: 'var(--chart-tooltip-label)' }}
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
