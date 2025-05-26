import React from "react";
import { TimeRange } from "../types";

interface DashboardHeaderProps {
  timeRange: TimeRange;
  onTimeRangeChange: (range: TimeRange) => void;
  refreshRate: number;
  onRefreshRateChange: (rate: number) => void;
}

export const DashboardHeader: React.FC<DashboardHeaderProps> = ({
  timeRange,
  onTimeRangeChange,
  refreshRate,
  onRefreshRateChange,
}) => {
  return (
    <header className="flex flex-col md:flex-row justify-between items-center pb-4 border-b border-gray-200">
      <h1 className="text-3xl font-bold" style={{ color: "#e81899" }}>
        {" "}
        {/* Updated Taiko Pink */}
        Taiko Masaya Testnet
      </h1>
      <div className="flex items-center space-x-2 mt-4 md:mt-0">
        <TimeRangeSelector
          currentTimeRange={timeRange}
          onTimeRangeChange={onTimeRangeChange}
        />
        <RefreshRateInput
          refreshRate={refreshRate}
          onRefreshRateChange={onRefreshRateChange}
        />
        {/* Export button removed as per request */}
      </div>
    </header>
  );
};

interface TimeRangeSelectorProps {
  currentTimeRange: TimeRange;
  onTimeRangeChange: (range: TimeRange) => void;
}

const TimeRangeSelector: React.FC<TimeRangeSelectorProps> = ({
  currentTimeRange,
  onTimeRangeChange,
}) => {
  const ranges: TimeRange[] = ["1h", "24h", "7d"];

  return (
    <div className="flex space-x-1 bg-gray-200 p-0.5 rounded-md">
      {ranges.map((range) => (
        <button
          key={range}
          onClick={() => onTimeRangeChange(range)}
          className={`px-3 py-1.5 text-sm font-medium rounded-md transition-colors
            ${currentTimeRange === range ? "bg-white text-[#e81899] shadow-sm" : "text-gray-600 hover:bg-gray-100"}`} /* Updated Taiko Pink for active button */
        >
          {range.toUpperCase()}
        </button>
      ))}
    </div>
  );
};

interface RefreshRateInputProps {
  refreshRate: number;
  onRefreshRateChange: (rate: number) => void;
}

const RefreshRateInput: React.FC<RefreshRateInputProps> = ({
  refreshRate,
  onRefreshRateChange,
}) => {
  const options = [
    { label: "10s", value: 10_000 },
    { label: "60s", value: 60_000 },
    { label: "5 min", value: 5 * 60_000 },
    { label: "10 min", value: 10 * 60_000 },
  ];

  const handleChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const value = Number(e.target.value);
    onRefreshRateChange(value);
  };

  return (
    <div className="flex items-center space-x-1">
      <label htmlFor="refreshRate" className="text-sm text-gray-600">
        Refresh
      </label>
      <select
        id="refreshRate"
        value={refreshRate}
        onChange={handleChange}
        className="p-1 border rounded-md text-sm"
      >
        {options.map(({ label, value }) => (
          <option key={value} value={value}>
            {label}
          </option>
        ))}
      </select>
    </div>
  );
};
