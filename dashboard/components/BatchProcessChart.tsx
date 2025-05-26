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

interface BatchProcessChartProps {
  data: TimeSeriesData[];
  lineColor: string;
}

export const BatchProcessChart: React.FC<BatchProcessChartProps> = ({
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

  const maxSeconds = Math.max(...data.map((d) => d.value));
  const unit =
    maxSeconds >= 7200 ? "hours" : maxSeconds >= 120 ? "minutes" : "seconds";

  const formatValue = (value: number) => {
    if (unit === "hours") {
      return `${Number((value / 3600).toFixed(2))} hours`;
    }
    if (unit === "minutes") {
      return `${Number((value / 60).toFixed(2))} minutes`;
    }
    return `${Math.round(value)} seconds`;
  };

  return (
    <ResponsiveContainer width="100%" height="100%">
      <LineChart
        data={data}
        margin={{ top: 5, right: 30, left: 20, bottom: 20 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
        <XAxis
          dataKey="name"
          stroke="#666666"
          fontSize={12}
          label={{
            value: "Batch ID",
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
          tickFormatter={(v) => {
            if (unit === "hours") return Number((v / 3600).toFixed(2));
            if (unit === "minutes") return Number((v / 60).toFixed(2));
            return v.toString();
          }}
          label={{
            value:
              unit === "hours"
                ? "Hours"
                : unit === "minutes"
                  ? "Minutes"
                  : "Seconds",
            angle: -90,
            position: "insideLeft",
            offset: -5,
            fontSize: 10,
            fill: "#666666",
          }}
        />
        <Tooltip
          formatter={(value: number) => [formatValue(value)]}
          labelFormatter={(label) => `Batch ${label}`}
          contentStyle={{
            backgroundColor: "rgba(255, 255, 255, 0.9)",
            borderColor: lineColor,
            borderRadius: "4px",
          }}
          labelStyle={{ color: "#333", fontWeight: "bold" }}
        />
        <Legend
          verticalAlign="bottom"
          align="right"
          wrapperStyle={{ right: 20, bottom: 0 }}
        />
        <Line
          type="monotone"
          dataKey="value"
          stroke={lineColor}
          strokeWidth={2}
          dot={{ r: 3 }}
          activeDot={{ r: 6 }}
          name={
            unit === "hours"
              ? "Time (hours)"
              : unit === "minutes"
                ? "Time (minutes)"
                : "Time (seconds)"
          }
        />
      </LineChart>
    </ResponsiveContainer>
  );
};
