import React from 'react';
import {
  LineChart,
  Line,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import { useIsMobile } from '../hooks/useIsMobile';
import { TimeSeriesData } from '../types';
import {
  formatDecimal,
  formatInterval,
  computeIntervalFlags,
  formatDateTime,
} from '../utils';

interface BlockTimeChartProps {
  data: TimeSeriesData[];
  lineColor: string;
  histogram?: boolean;
  seconds?: boolean;
}

const BlockTimeChartComponent: React.FC<BlockTimeChartProps> = ({
  data,
  lineColor,
  histogram = false,
  seconds = false,
}) => {
  const isMobile = useIsMobile();
  if (!data || data.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }
  const { showHours, showMinutes } = computeIntervalFlags(data, seconds);
  const ChartComponent = histogram ? BarChart : LineChart;
  return (
    <ResponsiveContainer width="100%" height="100%">
      <ChartComponent
        data={data}
        margin={{ top: 5, right: 20, left: 20, bottom: 40 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
        <XAxis
          dataKey="value"
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
          padding={{ left: isMobile ? 5 : 10, right: isMobile ? 5 : 10 }}
        />
        <YAxis
          stroke="#666666"
          fontSize={12}
          domain={['auto', 'auto']}
          tickFormatter={(v) =>
            showHours
              ? String(Number(formatDecimal(v / (seconds ? 3600 : 3600000))))
              : showMinutes
                ? String(Number(formatDecimal(v / (seconds ? 60 : 60000))))
                : String(Number(formatDecimal(seconds ? v : v / 1000)))
          }
          label={{
            value: showHours
              ? 'Avg Hours'
              : showMinutes
                ? 'Avg Minutes'
                : 'Avg Seconds',
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
          formatter={(value: number) => [
            formatInterval(
              seconds ? value : value / 1000,
              showHours,
              showMinutes,
            ),
          ]}
          contentStyle={{
            backgroundColor: 'rgba(255, 255, 255, 0.8)',
            borderColor: lineColor,
          }}
          labelStyle={{ color: '#333' }}
        />
        {histogram ? (
          <Bar dataKey="timestamp" fill={lineColor} name="Avg Time" />
        ) : (
          <Line
            type="monotone"
            dataKey="timestamp"
            stroke={lineColor}
            strokeWidth={2}
            dot={false}
            activeDot={data.length <= 100 ? { r: 6 } : false}
            name="Avg Time"
          />
        )}
      </ChartComponent>
    </ResponsiveContainer>
  );
};

export const BlockTimeChart = React.memo(BlockTimeChartComponent);
