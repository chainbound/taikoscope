import React, { useMemo } from 'react';
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
import { formatDateTime } from '../utils';
import type { BlockTransaction } from '../services/apiService';

interface BlockTxChartProps {
  data: BlockTransaction[];
  lineColor: string;
}

const BlockTxChartComponent: React.FC<BlockTxChartProps> = ({
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
  const sortedData = useMemo(
    () => [...data].sort((a, b) => a.block - b.block),
    [data],
  );
  const isMobile = useIsMobile();
  return (
    <ResponsiveContainer width="100%" height="100%">
      <LineChart
        data={sortedData}
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
            value: 'Avg Tx Count',
            angle: -90,
            position: 'insideLeft',
            offset: -16,
            fontSize: 10,
            fill: '#666666',
          }}
        />
        <Tooltip
          labelFormatter={(label: number, payload) => {
            const ts = payload?.[0]?.payload?.blockTime;
            const timeStr = ts ? formatDateTime(ts) : '';
            return `Block ${label.toLocaleString()} (${timeStr})`;
          }}
          formatter={(value: number) => [value.toLocaleString(), 'avg txs']}
          contentStyle={{
            backgroundColor: 'rgba(255, 255, 255, 0.8)',
            borderColor: lineColor,
          }}
          labelStyle={{ color: '#333' }}
        />
        <Line
          type="monotone"
          dataKey="txs"
          stroke={lineColor}
          strokeWidth={2}
          dot={false}
          activeDot={sortedData.length <= 100 ? { r: 6 } : false}
          name="Avg Txs"
        />
      </LineChart>
    </ResponsiveContainer>
  );
};

export const BlockTxChart = React.memo(BlockTxChartComponent);
