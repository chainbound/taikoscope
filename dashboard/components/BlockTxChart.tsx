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
  return (
    <ResponsiveContainer width="100%" height="100%">
      <BarChart
        data={sortedData}
        margin={{ top: 5, right: 70, left: 80, bottom: 40 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
        <XAxis
          dataKey="block"
          type="number"
          domain={['dataMin', 'dataMax']}
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
