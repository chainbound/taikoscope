import React from 'react';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from 'recharts';
import type { BatchBlobCount } from '../services/apiService';

interface BlobsPerBatchChartProps {
  data: BatchBlobCount[];
  barColor: string;
}

export const BlobsPerBatchChart: React.FC<BlobsPerBatchChartProps> = ({ data, barColor }) => {
  if (!data || data.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }

  return (
    <ResponsiveContainer width="100%" height="100%">
      <BarChart data={data} margin={{ top: 5, right: 30, left: 20, bottom: 50 }}>
        <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
        <XAxis
          dataKey="batch"
          tickFormatter={(v: number) => v.toLocaleString()}
          stroke="#666666"
          fontSize={12}
          label={{
            value: 'Batch',
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
          tickFormatter={(v: number) => v.toLocaleString()}
          label={{
            value: 'Blobs',
            angle: -90,
            position: 'insideLeft',
            offset: -16,
            fontSize: 10,
            fill: '#666666',
          }}
        />
        <Tooltip
          labelFormatter={(label: number) => `Batch ${label.toLocaleString()}`}
          formatter={(value: number) => [value.toLocaleString(), 'blobs']}
          contentStyle={{ backgroundColor: 'rgba(255, 255, 255, 0.8)', borderColor: barColor }}
          labelStyle={{ color: '#333' }}
        />
        <Legend verticalAlign="bottom" align="right" wrapperStyle={{ right: 20, bottom: 0 }} />
        <Bar dataKey="blobs" fill={barColor} name="Blobs" />
      </BarChart>
    </ResponsiveContainer>
  );
};
