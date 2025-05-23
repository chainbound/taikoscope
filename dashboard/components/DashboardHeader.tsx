
import React from 'react';
import { TimeRange } from '../types';

interface DashboardHeaderProps {
  timeRange: TimeRange;
  onTimeRangeChange: (range: TimeRange) => void;
}

export const DashboardHeader: React.FC<DashboardHeaderProps> = ({ timeRange, onTimeRangeChange }) => {
  return (
    <header className="flex flex-col md:flex-row justify-between items-center pb-4 border-b border-gray-200">
      <h1 className="text-3xl font-bold" style={{ color: '#e81899' }}> {/* Updated Taiko Pink */}
        Taiko Masaya Testnet
      </h1>
      <div className="flex items-center space-x-2 mt-4 md:mt-0">
        <TimeRangeSelector currentTimeRange={timeRange} onTimeRangeChange={onTimeRangeChange} />
        {/* Export button removed as per request */}
      </div>
    </header>
  );
};

interface TimeRangeSelectorProps {
  currentTimeRange: TimeRange;
  onTimeRangeChange: (range: TimeRange) => void;
}

const TimeRangeSelector: React.FC<TimeRangeSelectorProps> = ({ currentTimeRange, onTimeRangeChange }) => {
  const ranges: TimeRange[] = ['1h', '24h'];

  return (
    <div className="flex space-x-1 bg-gray-200 p-0.5 rounded-md">
      {ranges.map((range) => (
        <button
          key={range}
          onClick={() => onTimeRangeChange(range)}
          className={`px-3 py-1.5 text-sm font-medium rounded-md transition-colors
            ${currentTimeRange === range ? 'bg-white text-[#e81899] shadow-sm' : 'text-gray-600 hover:bg-gray-100'}`} /* Updated Taiko Pink for active button */
        >
          {range.toUpperCase()}
        </button>
      ))}
    </div>
  );
};