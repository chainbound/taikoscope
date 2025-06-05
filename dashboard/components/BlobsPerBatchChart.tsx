import React from 'react';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import type { BatchBlobCount } from '../services/apiService';

interface BlobsPerBatchChartProps {
  data: BatchBlobCount[];
  barColor: string;
}

const BlobsPerBatchChartComponent: React.FC<BlobsPerBatchChartProps> = ({
  data,
  barColor,
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
      <BarChart
        data={data}
        margin={{ top: 5, right: 70, left: 80, bottom: 40 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
        <XAxis
          dataKey="block"
          tickFormatter={(v: number) => v.toLocaleString()}
          stroke="#666666"
          fontSize={12}
          label={{
            value: 'L1 Block',
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
          domain={[0, 'auto']}
          allowDecimals={false}
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
          labelFormatter={(label: number, payload) => {
            const batch = payload?.[0]?.payload.batch as number;
            return `Block ${label.toLocaleString()} (Batch ${batch.toLocaleString()})`;
          }}
          formatter={(value: number) => [value.toLocaleString(), 'blobs']}
          contentStyle={{
            backgroundColor: 'rgba(255, 255, 255, 0.8)',
            borderColor: barColor,
          }}
          labelStyle={{ color: '#333' }}
        />
        <Bar dataKey="blobs" fill={barColor} name="Blobs" />
      </BarChart>
    </ResponsiveContainer>
  );
};

export const BlobsPerBatchChart = React.memo(BlobsPerBatchChartComponent);
