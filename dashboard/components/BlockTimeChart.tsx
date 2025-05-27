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
import { formatDecimal } from "../utils";

interface BlockTimeChartProps {
  data: TimeSeriesData[];
  lineColor: string;
}

const formatInterval = (ms: number, showMinutes: boolean): string => {
  return showMinutes
    ? `${formatDecimal(ms / 60000)} minutes`
    : `${Math.round(ms / 1000)} seconds`;
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
  const showMinutes = data.some((d) => d.timestamp >= 120000);
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
          tickFormatter={(v) =>
            showMinutes
              ? Number(formatDecimal(v / 60000))
              : Math.round(v / 1000)
          }
          label={{
            value: showMinutes ? "Minutes" : "Seconds",
            angle: -90,
            position: "insideLeft",
            offset: -16,
            fontSize: 10,
            fill: "#666666",
          }}
        />
        <Tooltip
          labelFormatter={(label: number) => `Block ${label.toLocaleString()}`}
          formatter={(value: number) => [formatInterval(value, showMinutes)]}
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
          dot={data.length > 100 ? false : { r: 3 }}
          activeDot={data.length > 100 ? false : { r: 6 }}
          name="Time"
        />
      </LineChart>
    </ResponsiveContainer>
  );
};
