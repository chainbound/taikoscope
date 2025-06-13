import React from 'react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import { useEthPrice } from '../services/priceService';
import { MetricData } from '../types';
import { findMetricValue } from '../utils';

interface ProfitabilityChartProps {
  metrics: MetricData[];
  hours: number; // hours represented by the metric values
  cloudCost: number; // monthly cloud cost from calculator
}

export const ProfitabilityChart: React.FC<ProfitabilityChartProps> = ({
  metrics,
  hours,
  cloudCost,
}) => {
  const priorityStr = findMetricValue(metrics, 'priority fee');
  const baseStr = findMetricValue(metrics, 'base fee');
  const priorityFee = parseFloat(priorityStr.replace(/[^0-9.]/g, '')) || null;
  const baseFee = parseFloat(baseStr.replace(/[^0-9.]/g, '')) || null;
  const HOURS_IN_MONTH = 30 * 24;
  const scaledCloudCost = (cloudCost / HOURS_IN_MONTH) * hours;
  const l2TxFee =
    priorityFee != null && baseFee != null ? priorityFee + baseFee : null;
  const { data: ethPrice = 0 } = useEthPrice();

  if (l2TxFee == null || hours === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }

  const profitPerHour = (l2TxFee * ethPrice - scaledCloudCost) / hours;

  const data = Array.from({ length: 12 }, (_, i) => {
    const month = i + 1;
    const hoursInMonth = 30 * 24 * month;
    return {
      month,
      profit: profitPerHour * hoursInMonth,
    };
  });

  return (
    <ResponsiveContainer width="100%" height={240}>
      <LineChart
        data={data}
        margin={{ top: 5, right: 40, left: 20, bottom: 40 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
        <XAxis
          dataKey="month"
          stroke="#666666"
          fontSize={12}
          label={{
            value: 'Month',
            position: 'insideBottom',
            offset: -10,
            fontSize: 10,
            fill: '#666666',
          }}
        />
        <YAxis
          stroke="#666666"
          fontSize={12}
          domain={[0, 'auto']}
          tickFormatter={(v: number) => `$${v.toFixed(0)}`}
          label={{
            value: 'Profit (USD)',
            angle: -90,
            position: 'insideLeft',
            offset: -16,
            fontSize: 10,
            fill: '#666666',
          }}
        />
        <Tooltip
          labelFormatter={(v: number) => `Month ${v}`}
          formatter={(value: number) => [`$${value.toFixed(2)}`, 'Profit']}
          contentStyle={{
            backgroundColor: 'rgba(255,255,255,0.8)',
            borderColor: '#8884d8',
          }}
          labelStyle={{ color: '#333' }}
        />
        <Line
          type="monotone"
          dataKey="profit"
          stroke="#8884d8"
          strokeWidth={2}
          dot={false}
          name="Profit"
        />
      </LineChart>
    </ResponsiveContainer>
  );
};
