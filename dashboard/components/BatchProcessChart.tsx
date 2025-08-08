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
import {
  formatDecimal,
  computeBatchDurationFlags,
  formatHoursMinutes,
  formatSeconds,
} from '../utils';

interface BatchProcessChartProps {
  data: TimeSeriesData[];
  lineColor: string;
}

const BatchProcessChartComponent: React.FC<BatchProcessChartProps> = ({
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

  const { showHours, showMinutes } = computeBatchDurationFlags(data);
  const formatValue = (value: number) => formatSeconds(value);

  return (
    <ResponsiveContainer width="100%" height="100%">
      <LineChart
        data={data}
        margin={{ top: 5, right: 20, left: 20, bottom: 40 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="var(--chart-grid)" />
        <XAxis
          dataKey="name"
          tickFormatter={(v) => Number(v).toLocaleString()}
          stroke="var(--chart-tick)"
          fontSize={12}
          label={{
            value: 'Batch ID',
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
          tickFormatter={(v) =>
            showHours
              ? formatHoursMinutes(v)
              : showMinutes
                ? Number(formatDecimal(v / 60))
                : v.toString()
          }
          label={{
            value: showHours
              ? 'Avg Hours'
              : showMinutes
                ? 'Avg Minutes'
                : 'Avg Seconds',
            angle: -90,
            position: 'insideLeft',
            offset: -5,
            fontSize: 10,
            fill: 'var(--chart-tick)',
          }}
        />
        <Tooltip
          formatter={(value: number) => [formatValue(value)]}
          labelFormatter={(label) => `Batch ${Number(label).toLocaleString()}`}
          contentStyle={{
            backgroundColor: 'var(--chart-tooltip-bg)',
            borderColor: lineColor,
            borderRadius: '4px',
          }}
          labelStyle={{ color: 'var(--chart-tooltip-label)', fontWeight: 'bold' }}
        />
        <Line
          type="monotone"
          dataKey="value"
          stroke={lineColor}
          strokeWidth={2}
          dot={false}
          activeDot={data.length <= 100 ? { r: 6 } : false}
          name="Avg Time"
        />
      </LineChart>
    </ResponsiveContainer>
  );
};

export const BatchProcessChart = React.memo(BatchProcessChartComponent);
