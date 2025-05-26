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
    <header className="flex flex-col md:flex-row justify-between items-center pb-6 mb-6 border-b border-gray-700">
      <h1 className="text-4xl font-extrabold tracking-tight bg-gradient-to-r from-[#e81899] via-pink-500 to-purple-500 bg-clip-text text-transparent">
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
    <div className="flex space-x-1 bg-gray-800 p-0.5 rounded-md">
      {ranges.map((range) => (
        <button
          key={range}
          onClick={() => onTimeRangeChange(range)}
          className={`px-3 py-1.5 text-sm font-medium rounded-md transition-colors
            ${currentTimeRange === range ? "bg-[#e81899] text-white" : "text-gray-300 hover:bg-gray-700"}`}
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
      <label htmlFor="refreshRate" className="text-sm text-gray-300">
        Refresh
      </label>
      <select
        id="refreshRate"
        value={refreshRate}
        onChange={handleChange}
        className="p-1 border border-gray-700 rounded-md text-sm bg-gray-900 text-gray-100"
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
