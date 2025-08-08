import React, { useMemo } from 'react';
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
  const isMobile = useIsMobile();
  const sortedData = useMemo(
    () => (data ? [...data].sort((a, b) => a.block_number - b.block_number) : []),
    [data],
  );
  if (sortedData.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }
  return (
    <ResponsiveContainer width="100%" height="100%">
      <LineChart
        data={sortedData}
        margin={{ top: 5, right: 20, left: 20, bottom: 40 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="var(--chart-grid)" />
        <XAxis
          dataKey="block_number"
          tickFormatter={(v: number) => v.toLocaleString()}
          stroke="var(--chart-tick)"
          fontSize={12}
          label={{
            value: 'L2 Block Number',
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
          domain={[0, 'auto']}
          allowDecimals={false}
          tickFormatter={(v: number) => v.toLocaleString()}
          label={{
            value: 'Avg Tx Count',
            angle: -90,
            position: 'insideLeft',
            offset: -16,
            fontSize: 10,
            fill: 'var(--chart-tick)',
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
            backgroundColor: 'var(--chart-tooltip-bg)',
            borderColor: lineColor,
          }}
          labelStyle={{ color: 'var(--chart-tooltip-label)' }}
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
