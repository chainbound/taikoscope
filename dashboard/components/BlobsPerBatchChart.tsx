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
import { useIsMobile } from '../hooks/useIsMobile';
import type { BatchBlobCount } from '../services/apiService';

interface BlobsPerBatchChartProps {
  data: BatchBlobCount[];
  barColor: string;
}

const BlobsPerBatchChartComponent: React.FC<BlobsPerBatchChartProps> = ({
  data,
  barColor,
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
      <BarChart
        data={data}
        margin={{ top: 5, right: 20, left: 20, bottom: 40 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
        <XAxis
          dataKey="batch"
          tickFormatter={(v: number) => v.toLocaleString()}
          stroke="#666666"
          fontSize={12}
          label={{
            value: 'Batch',
            position: 'insideBottom',
            offset: -35,
            fontSize: 10,
            fill: '#666666',
          }}
          padding={{ left: isMobile ? 5 : 10, right: isMobile ? 5 : 10 }}
        />
        <YAxis
          stroke="#666666"
          fontSize={12}
          domain={[0, 'auto']}
          tickFormatter={(v: number) => v.toLocaleString()}
          label={{
            value: 'Avg Blobs',
            angle: -90,
            position: 'insideLeft',
            offset: -16,
            fontSize: 10,
            fill: '#666666',
          }}
        />
        <Tooltip
          labelFormatter={(label: number, payload) => {
            const block = payload?.[0]?.payload?.block_number as number;
            if (block && label) {
              return `Batch ${label.toLocaleString()} (Block ${block.toLocaleString()})`;
            }
            return label ? `Batch ${label.toLocaleString()}` : 'Unknown';
          }}
          formatter={(value: number) => [value.toLocaleString(), 'blobs']}
          contentStyle={{
            backgroundColor: 'rgba(255, 255, 255, 0.8)',
            borderColor: barColor,
          }}
          labelStyle={{ color: '#333' }}
        />
        <Bar dataKey="blobs" fill={barColor} name="Avg Blobs" />
      </BarChart>
    </ResponsiveContainer>
  );
};

export const BlobsPerBatchChart = React.memo(BlobsPerBatchChartComponent);
