import React from "react";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from "recharts";
import { TimeSeriesData } from "../types";

interface BlockTimeChartProps {
  data: TimeSeriesData[];
  lineColor: string;
}

const formatTimestampToTime = (timestamp: number): string => {
  const date = new Date(timestamp);
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
};

export const BlockTimeChart: React.FC<BlockTimeChartProps> = ({
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
  return (
    <ResponsiveContainer width="100%" height="100%">
      <LineChart
        data={data}
        margin={{ top: 5, right: 30, left: 20, bottom: 50 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
        <XAxis
          dataKey="value"
          tickFormatter={(v: number) => v.toLocaleString()}
          stroke="#666666"
          fontSize={12}
          label={{
            value: "Block Number",
            position: "insideBottom",
            offset: -10,
            fontSize: 10,
            fill: "#666666",
          }}
          padding={{ left: 10, right: 10 }}
        />
        <YAxis
          stroke="#666666"
          fontSize={12}
          domain={["auto", "auto"]}
          tickFormatter={formatTimestampToTime}
          label={{
            value: "Time",
            angle: -90,
            position: "insideLeft",
            offset: -16,
            fontSize: 10,
            fill: "#666666",
          }}
        />
        <Tooltip
          labelFormatter={(label: number) => `Block ${label.toLocaleString()}`}
          formatter={(value: number) => [formatTimestampToTime(value), "Time"]}
          contentStyle={{
            backgroundColor: "rgba(255, 255, 255, 0.8)",
            borderColor: lineColor,
          }}
          labelStyle={{ color: "#333" }}
        />
        <Legend
          verticalAlign="bottom"
          align="right"
          wrapperStyle={{ right: 20, bottom: 0 }}
        />
        <Line
          type="monotone"
          dataKey="timestamp"
          stroke={lineColor}
          strokeWidth={2}
          dot={{ r: 3 }}
          activeDot={{ r: 6 }}
          name="Time"
        />
      </LineChart>
    </ResponsiveContainer>
  );
};
