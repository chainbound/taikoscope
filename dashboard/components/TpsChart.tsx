import React from 'react';
import { useIsMobile } from '../hooks/useIsMobile';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';

export interface TpsData {
  block: number;
  tps: number;
}

interface TpsChartProps {
  data: TpsData[];
  lineColor: string;
}

const TpsChartComponent: React.FC<TpsChartProps> = ({ data, lineColor }) => {
  if (!data || data.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }
  const isMobile = useIsMobile();
  return (
    <ResponsiveContainer width="100%" height="100%">
      <LineChart
        data={data}
        margin={{
          top: 5,
          right: isMobile ? 10 : 20,
          left: isMobile ? 10 : 20,
          bottom: 40,
        }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
        <XAxis
          dataKey="block"
          tickFormatter={(v: number) => v.toLocaleString()}
          stroke="#666666"
          fontSize={12}
          label={{
            value: 'L2 Block Number',
            position: 'insideBottom',
            offset: -10,
            fontSize: 10,
            fill: '#666666',
          }}
          padding={{ left: 10, right: 10 }}
        />
        <YAxis
          stroke="#666666"
          fontSize={12}
          domain={[0, 'auto']}
          allowDecimals={false}
          label={{
            value: 'Avg TPS',
            angle: -90,
            position: 'insideLeft',
            offset: -16,
            fontSize: 10,
            fill: '#666666',
          }}
        />
        <Tooltip
          labelFormatter={(label: number) => `Block ${label.toLocaleString()}`}
          formatter={(value: number) => [value.toFixed(2), 'avg tps']}
          contentStyle={{
            backgroundColor: 'rgba(255, 255, 255, 0.8)',
            borderColor: lineColor,
          }}
          labelStyle={{ color: '#333' }}
        />
        <Line
          type="monotone"
          dataKey="tps"
          stroke={lineColor}
          strokeWidth={2}
          dot={false}
          name="Avg TPS"
        />
      </LineChart>
    </ResponsiveContainer>
  );
};

export const TpsChart = React.memo(TpsChartComponent);
