import React, { useMemo } from 'react';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import type { BlockTransaction } from '../services/apiService';

interface BlockTxChartProps {
  data: BlockTransaction[];
  barColor: string;
}

const BlockTxChartComponent: React.FC<BlockTxChartProps> = ({
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
  const sortedData = useMemo(
    () => [...data].sort((a, b) => a.block - b.block),
    [data],
  );

  // Calculate intelligent tick configuration based on data range
  const tickConfig = useMemo(() => {
    if (sortedData.length === 0) return {};
    
    const minBlock = sortedData[0].block;
    const maxBlock = sortedData[sortedData.length - 1].block;
    const range = maxBlock - minBlock;
    
    // For small ranges, show more ticks. For large ranges, show fewer ticks.
    let tickCount = 6; // Default
    if (range <= 10) tickCount = Math.max(2, range);
    else if (range <= 50) tickCount = 8;
    else if (range <= 200) tickCount = 6;
    else tickCount = 5;
    
    return { tickCount };
  }, [sortedData]);
  return (
    <ResponsiveContainer width="100%" height="100%">
      <BarChart
        data={sortedData}
        margin={{ top: 5, right: 70, left: 80, bottom: 40 }}
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
            offset: -35,
            fontSize: 10,
            fill: '#666666',
          }}
          padding={{ left: 10, right: 10 }}
          type="number"
          scale="linear"
          domain={['dataMin', 'dataMax']}
          tickCount={tickConfig.tickCount}
        />
        <YAxis
          stroke="#666666"
          fontSize={12}
          domain={[0, 'auto']}
          allowDecimals={false}
          tickFormatter={(v: number) => v.toLocaleString()}
          label={{
            value: 'Tx Count',
            angle: -90,
            position: 'insideLeft',
            offset: -16,
            fontSize: 10,
            fill: '#666666',
          }}
        />
        <Tooltip
          labelFormatter={(label: number) => `Block ${label.toLocaleString()}`}
          formatter={(value: number) => [value.toLocaleString(), 'txs']}
          contentStyle={{
            backgroundColor: 'rgba(255, 255, 255, 0.8)',
            borderColor: barColor,
          }}
          labelStyle={{ color: '#333' }}
        />
        <Bar dataKey="txs" fill={barColor} name="Txs" />
      </BarChart>
    </ResponsiveContainer>
  );
};

export const BlockTxChart = React.memo(BlockTxChartComponent);
