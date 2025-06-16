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
import {
  formatDecimal,
  formatBatchDuration,
  computeBatchDurationFlags,
} from '../utils';

interface BatchProcessChartProps {
  data: TimeSeriesData[];
  lineColor: string;
}

const BatchProcessChartComponent: React.FC<BatchProcessChartProps> = ({
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

  const { showHours, showMinutes } = computeBatchDurationFlags(data);
  const formatValue = (value: number) =>
    formatBatchDuration(value, showHours, showMinutes);

  return (
    <ResponsiveContainer width="100%" height="100%">
      <LineChart
        data={data}
        margin={{ top: 5, right: 70, left: 80, bottom: 40 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
        <XAxis
          dataKey="name"
          tickFormatter={(v) => Number(v).toLocaleString()}
          stroke="#666666"
          fontSize={12}
          label={{
            value: 'Batch ID',
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
          tickFormatter={(v) =>
            showHours
              ? Number(formatDecimal(v / 3600))
              : showMinutes
                ? Number(formatDecimal(v / 60))
                : v.toString()
          }
          label={{
            value: showHours ? 'Hours' : showMinutes ? 'Minutes' : 'Seconds',
            angle: -90,
            position: 'insideLeft',
            offset: -5,
            fontSize: 10,
            fill: '#666666',
          }}
        />
        <Tooltip
          formatter={(value: number) => [formatValue(value)]}
          labelFormatter={(label) => `Batch ${Number(label).toLocaleString()}`}
          contentStyle={{
            backgroundColor: 'rgba(255, 255, 255, 0.9)',
            borderColor: lineColor,
            borderRadius: '4px',
          }}
          labelStyle={{ color: '#333', fontWeight: 'bold' }}
        />
        <Line
          type="monotone"
          dataKey="value"
          stroke={lineColor}
          strokeWidth={2}
          dot={false}
          activeDot={data.length <= 100 ? { r: 6 } : false}
          name="Time"
        />
      </LineChart>
    </ResponsiveContainer>
  );
};

export const BatchProcessChart = React.memo(BatchProcessChartComponent);
