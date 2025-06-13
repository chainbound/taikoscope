import React from 'react';
import { TimeRange } from '../types';
import { RefreshCountdown } from './RefreshCountdown';
import { TAIKO_PINK } from '../theme';
import { isValidRefreshRate } from '../utils';
import { isValidTimeRange } from '../utils/timeRange';
import { useRouterNavigation } from '../hooks/useRouterNavigation';
import { useErrorHandler } from '../hooks/useErrorHandler';
import { showToast } from '../utils/toast';

interface ImportMetaEnv {
  readonly VITE_NETWORK_NAME?: string;
  readonly NETWORK_NAME?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

const metaEnv = (import.meta as ImportMeta).env;
const rawNetworkName =
  metaEnv?.VITE_NETWORK_NAME ?? metaEnv?.NETWORK_NAME ?? 'Masaya';
const NETWORK_NAME = rawNetworkName.charAt(0).toUpperCase() + rawNetworkName.slice(1).toLowerCase();
const DASHBOARD_TITLE = `Taikoscope ${NETWORK_NAME}`;

interface DashboardHeaderProps {
  timeRange: TimeRange;
  onTimeRangeChange: (range: TimeRange) => void;
  refreshRate: number;
  onRefreshRateChange: (rate: number) => void;
  lastRefresh: number;
  onManualRefresh: () => void;
  isTimeRangeChanging?: boolean;
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
  isTimeRangeChanging,
  sequencers,
  selectedSequencer,
  onSequencerChange,
}) => {
  const { navigateToDashboard } = useRouterNavigation();
  const { errorMessage } = useErrorHandler();
  React.useEffect(() => {
    if (errorMessage) {
      showToast(errorMessage);
    }
  }, [errorMessage]);
  return (
    <header className="flex flex-col md:flex-row justify-between items-center pb-4 border-b border-gray-200 dark:border-gray-700">
      <div className="flex items-baseline space-x-4">
        <h1
          className="text-3xl font-bold cursor-pointer hover:underline"
          style={{ color: TAIKO_PINK }}
          onClick={() => {
            navigateToDashboard(true);
          }}
        >
          {' '}
          {/* Updated Taiko Pink */}
          {DASHBOARD_TITLE}
        </h1>
      </div>
      <div className="flex items-center space-x-2 mt-4 md:mt-0">
        {/* Economics view is still supported via URL parameters, but the
            navigation button is hidden. */}
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
          isChanging={isTimeRangeChanging}
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
  isChanging?: boolean;
}

export const TimeRangeSelector: React.FC<TimeRangeSelectorProps> = ({
  currentTimeRange,
  onTimeRangeChange,
  isChanging,
}) => {
  const presetRanges: TimeRange[] = ['15m', '1h', '3h', '6h', '12h', '24h'];

const isCustom = !presetRanges.includes(currentTimeRange);
const [customValue, setCustomValue] = React.useState(
  isCustom ? currentTimeRange : '1h'
);


  const handleSelect = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const value = e.target.value;
    if (value === 'custom') {
      if (isValidTimeRange(customValue)) onTimeRangeChange(customValue);
    } else {
      onTimeRangeChange(value);
    }
  };

  const applyCustom = () => {
    if (isValidTimeRange(customValue)) {
      onTimeRangeChange(customValue);
    }
  };

  return (
    <div className="flex items-center space-x-1">
      {isChanging && (
        <div className="flex items-center px-2">
          <div className="animate-spin rounded-full h-3 w-3 border-b-2 border-current opacity-50" />
        </div>
      )}
      <select
        value={isCustom ? 'custom' : currentTimeRange}
        onChange={handleSelect}
        disabled={isChanging}
        className="p-1 border rounded-md text-sm bg-white dark:bg-gray-800"
      >
        {presetRanges.map((r) => (
          <option key={r} value={r}>
            {r}
          </option>
        ))}
        <option value="custom">Custom...</option>
      </select>
      {isCustom && (
        <>
          <input
            type="text"
            value={customValue}
            onChange={(e) => setCustomValue(e.target.value)}
            placeholder="e.g. 45m"
            className="p-1 border rounded-md text-sm w-20 bg-white dark:bg-gray-800"
          />
          <button
            onClick={applyCustom}
            disabled={isChanging || !isValidTimeRange(customValue)}
            className="px-2 py-1 text-sm rounded-md bg-gray-200 dark:bg-gray-700 disabled:opacity-50"
          >
            Apply
          </button>
        </>
      )}
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
    if (isValidRefreshRate(value)) {
      onRefreshRateChange(value);
    }
  };

  return (
    <div className="flex items-center space-x-1">
      <label
        htmlFor="refreshRate"
        className="text-sm text-gray-600 dark:text-gray-300"
      >
        Refresh
      </label>
      <select
        id="refreshRate"
        value={refreshRate}
        onChange={handleChange}
        className="p-1 border border-gray-300 dark:border-gray-600 rounded-md text-sm bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100"
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
  const sorted = React.useMemo(
    () =>
      [...sequencers]
        .filter((s) => s.toLowerCase() !== 'all sequencers')
        .sort(),
    [sequencers],
  );

  return (
    <select
      value={value ?? ''}
      onChange={(e) => onChange(e.target.value || null)}
      className="p-1 border rounded-md text-sm"
    >
      <option value="">All Sequencers</option>
      {sorted.map((s) => (
        <option key={s} value={s}>
          {s}
        </option>
      ))}
    </select>
  );
};
