import React from 'react';
import { TimeRange } from '../types';
import { RefreshCountdown } from './RefreshCountdown';
import { TAIKO_PINK } from '../theme';

const metaEnv = (import.meta as any).env as ImportMetaEnv | undefined;
const NETWORK_NAME =
  metaEnv?.VITE_NETWORK_NAME ?? metaEnv?.NETWORK_NAME ?? 'Taiko Masaya Testnet';

interface DashboardHeaderProps {
  timeRange: TimeRange;
  onTimeRangeChange: (range: TimeRange) => void;
  refreshRate: number;
  onRefreshRateChange: (rate: number) => void;
  lastRefresh: number;
  onManualRefresh: () => void;
  sequencers: string[];
  selectedSequencer: string | null;
  onSequencerChange: (seq: string | null) => void;
}

export const DashboardHeader: React.FC<DashboardHeaderProps> = ({
  timeRange,
  onTimeRangeChange,
  refreshRate,
  onRefreshRateChange,
  lastRefresh,
  onManualRefresh,
  sequencers,
  selectedSequencer,
  onSequencerChange,
}) => {
  return (
    <header className="flex flex-col md:flex-row justify-between items-center pb-4 border-b border-gray-200">
      <div className="flex items-baseline space-x-4">
        <h1
          className="text-3xl font-bold cursor-pointer hover:underline"
          style={{ color: TAIKO_PINK }}
          onClick={() => {
            try {
              const url = new URL(window.location.href);
              url.searchParams.delete('view');
              url.searchParams.delete('table');
              url.searchParams.delete('address');
              url.searchParams.delete('page');
              url.searchParams.delete('start');
              url.searchParams.delete('end');
              window.history.pushState(null, '', url.toString());
            } catch (err) {
              console.error('Failed to navigate to home:', err);
              // Fallback: reload page
              window.location.href = window.location.pathname;
            }
          }}
        >
          {' '}
          {/* Updated Taiko Pink */}
          {NETWORK_NAME}
        </h1>
      </div>
      <div className="flex items-center space-x-2 mt-4 md:mt-0">
        <button
          onClick={() => {
            try {
              const url = new URL(window.location.href);
              if (url.searchParams.get('view') === 'economics') {
                url.searchParams.delete('view');
              } else {
                url.searchParams.set('view', 'economics');
              }
              url.searchParams.delete('table');
              window.history.pushState(null, '', url.toString());
            } catch (err) {
              console.error('Failed to toggle economics view:', err);
              // Fallback: just reload the page with economics parameter
              const fallbackUrl = new URL(window.location.href);
              try {
                fallbackUrl.searchParams.set('view', 'economics');
                window.location.href = fallbackUrl.toString();
              } catch (fallbackErr) {
                console.error('Fallback navigation also failed:', fallbackErr);
              }
            }
          }}
          className="text-sm hover:underline"
          style={{ color: TAIKO_PINK }}
        >
          Economics
        </button>
        <a
          href="https://taikoscope.instatus.com/"
          target="_blank"
          rel="noopener noreferrer"
          className="text-sm hover:underline"
          style={{ color: TAIKO_PINK }}
        >
          Status Page
        </a>
        <TimeRangeSelector
          currentTimeRange={timeRange}
          onTimeRangeChange={onTimeRangeChange}
        />
        <RefreshRateInput
          refreshRate={refreshRate}
          onRefreshRateChange={onRefreshRateChange}
        />
        <RefreshCountdown
          refreshRate={refreshRate}
          lastRefresh={lastRefresh}
          onRefresh={onManualRefresh}
        />
        <SequencerSelector
          sequencers={sequencers}
          value={selectedSequencer}
          onChange={onSequencerChange}
        />
        {/* Export button removed as per request */}
      </div>
    </header>
  );
};

export interface TimeRangeSelectorProps {
  currentTimeRange: TimeRange;
  onTimeRangeChange: (range: TimeRange) => void;
}

export const TimeRangeSelector: React.FC<TimeRangeSelectorProps> = ({
  currentTimeRange,
  onTimeRangeChange,
}) => {
  const ranges: TimeRange[] = ['1h', '24h', '7d'];

  return (
    <div className="flex space-x-1 bg-gray-200 p-0.5 rounded-md">
      {ranges.map((range) => (
        <button
          key={range}
          onClick={() => onTimeRangeChange(range)}
          className={`px-3 py-1.5 text-sm font-medium rounded-md transition-colors
            ${currentTimeRange === range ? 'bg-white shadow-sm' : 'text-gray-600 hover:bg-gray-100'}`} /* Updated Taiko Pink for active button */
          style={currentTimeRange === range ? { color: TAIKO_PINK } : undefined}
        >
          {range.toUpperCase()}
        </button>
      ))}
    </div>
  );
};

export interface RefreshRateInputProps {
  refreshRate: number;
  onRefreshRateChange: (rate: number) => void;
}

export const RefreshRateInput: React.FC<RefreshRateInputProps> = ({
  refreshRate,
  onRefreshRateChange,
}) => {
  const options = [
    { label: '60s', value: 60_000 },
    { label: '5 min', value: 5 * 60_000 },
    { label: '10 min', value: 10 * 60_000 },
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

export interface SequencerSelectorProps {
  sequencers: string[];
  value: string | null;
  onChange: (seq: string | null) => void;
}

export const SequencerSelector: React.FC<SequencerSelectorProps> = ({
  sequencers,
  value,
  onChange,
}) => {
  return (
    <select
      value={value ?? ''}
      onChange={(e) => onChange(e.target.value || null)}
      className="p-1 border rounded-md text-sm"
    >
      <option value="">All Sequencers</option>
      {sequencers.map((s) => (
        <option key={s} value={s}>
          {s}
        </option>
      ))}
    </select>
  );
};
