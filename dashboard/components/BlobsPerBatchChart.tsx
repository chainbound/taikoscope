import React, { useState, useEffect } from 'react';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Brush,
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
      <BarChart
        data={data}
        margin={{ top: 5, right: 70, left: 20, bottom: 40 }}
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
          labelFormatter={(label: number) => `Batch ${label.toLocaleString()}`}
          formatter={(value: number) => [value.toLocaleString(), 'blobs']}
          contentStyle={{
            backgroundColor: 'rgba(255, 255, 255, 0.8)',
            borderColor: barColor,
          }}
          labelStyle={{ color: '#333' }}
        />
        <Bar dataKey="blobs" fill={barColor} name="Blobs" />
        <Brush
          dataKey="batch"
          height={20}
          stroke={barColor}
          padding={{ left: 20, right: 20 }}
          startIndex={clampedRange.startIndex}
          endIndex={clampedRange.endIndex}
          onChange={handleBrushChange}
        />
      </BarChart>
    </ResponsiveContainer>
  );
};

export const BlobsPerBatchChart = React.memo(BlobsPerBatchChartComponent);
