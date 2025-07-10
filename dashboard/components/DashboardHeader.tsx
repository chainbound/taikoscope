import React from 'react';
import { TimeRange } from '../types';
import { RefreshCountdown } from './RefreshCountdown';
import { TAIKO_PINK } from '../theme';
import { isValidRefreshRate } from '../utils';
import { isValidTimeRange, formatTimeRangeDisplay } from '../utils/timeRange';
import { useRouterNavigation } from '../hooks/useRouterNavigation';
import { DEFAULT_VIEW } from '../constants';
import { useErrorHandler } from '../hooks/useErrorHandler';
import { useSearchParams } from 'react-router-dom';
import { showToast } from '../utils/toast';
import { DayPicker } from 'react-day-picker';
import * as Popover from '@radix-ui/react-popover';

interface ImportMetaEnv {
  readonly VITE_NETWORK_NAME?: string;
  readonly NETWORK_NAME?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

const metaEnv = (import.meta as ImportMeta).env;
const rawNetworkName =
  metaEnv?.VITE_NETWORK_NAME ?? metaEnv?.NETWORK_NAME ?? 'Hekla';
const NETWORK_NAME =
  rawNetworkName.charAt(0).toUpperCase() +
  rawNetworkName.slice(1).toLowerCase();
const DASHBOARD_TITLE = `Taikoscope ${NETWORK_NAME}`;

interface DashboardHeaderProps {
  timeRange: TimeRange;
  onTimeRangeChange: (range: TimeRange) => void;
  refreshRate: number;
  onRefreshRateChange: (rate: number) => void;
  lastRefresh: number;
  onManualRefresh: () => void;
  isTimeRangeChanging?: boolean;
}

export const DashboardHeader: React.FC<DashboardHeaderProps> = ({
  timeRange,
  onTimeRangeChange,
  refreshRate,
  onRefreshRateChange,
  lastRefresh,
  onManualRefresh,
  isTimeRangeChanging,
}) => {
  const { updateSearchParams } = useRouterNavigation();
  const { errorMessage } = useErrorHandler();
  const [searchParams] = useSearchParams();
  const viewParam = searchParams.get('view') ?? DEFAULT_VIEW;
  React.useEffect(() => {
    if (errorMessage) {
      showToast(errorMessage);
    }
  }, [errorMessage]);
  return (
    <header className="flex flex-col md:flex-row justify-between items-center pb-4 border-b border-gray-200 dark:border-gray-700">
      <div className="flex items-baseline space-x-4">
        <h1
          className="text-3xl font-bold"
          style={{ color: TAIKO_PINK }}
        >
          {' '}
          {/* Updated Taiko Pink */}
          {DASHBOARD_TITLE}
        </h1>
      </div>
      <div className="flex flex-wrap items-center gap-2 mt-4 md:mt-0 justify-center md:justify-end">
        <div className="flex gap-2">
          {[
            { view: 'economics', label: 'Economics' },
            { view: 'performance', label: 'Performance' },
            { view: 'health', label: 'Health' },
          ].map((tab) => (
            <button
              key={tab.view}
              onClick={() =>
                updateSearchParams({ view: tab.view, table: null })
              }
              className={`px-2 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded-md ${viewParam === tab.view ? 'bg-gray-200 dark:bg-gray-700' : 'bg-white dark:bg-gray-800 hover:bg-gray-100 dark:hover:bg-gray-700'}`}
              style={{ color: TAIKO_PINK }}
            >
              {tab.label}
            </button>
          ))}
        </div>
        <a
          href="https://status.taiko.xyz/"
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
        {/* Sequencer filter removed */}
        {/* Dark mode toggle removed as per request */}
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
  const { updateSearchParams } = useRouterNavigation();
  const presetRanges: TimeRange[] = [
    '15m',
    '1h',
    '3h',
    '6h',
    '12h',
    '24h',
    '7d',
  ];
  const isCustom = /^\d+-\d+$/.test(currentTimeRange);
  const [open, setOpen] = React.useState(false);
  const [date, setDate] = React.useState<Date | undefined>(() => {
    if (isCustom) {
      const [s, e] = currentTimeRange
        .split('-')
        .map((t) => new Date(Number(t)));
      if (s.toDateString() === e.toDateString()) return s;
    }
    return undefined;
  });
  const [fromTime, setFromTime] = React.useState('');
  const [toTime, setToTime] = React.useState('');

  const buttonLabel = React.useMemo(
    () =>
      isCustom ? formatTimeRangeDisplay(currentTimeRange) : currentTimeRange,
    [currentTimeRange, isCustom],
  );

  const customTooltip = React.useMemo(() => {
    if (!isCustom) return undefined;
    const [s, e] = currentTimeRange.split('-').map((t) => new Date(Number(t)));
    const fmt = (d: Date) =>
      `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(
        d.getDate(),
      ).padStart(2, '0')} ${d.getHours().toString().padStart(2, '0')}:${d
        .getMinutes()
        .toString()
        .padStart(2, '0')}`;
    return `From ${fmt(s)} to ${fmt(e)}`;
  }, [currentTimeRange, isCustom]);

  React.useEffect(() => {
    if (isCustom) {
      const [s, e] = currentTimeRange
        .split('-')
        .map((t) => new Date(Number(t)));
      if (s.toDateString() === e.toDateString()) {
        setDate(s);
        setFromTime(
          `${s.getHours().toString().padStart(2, '0')}:${s
            .getMinutes()
            .toString()
            .padStart(2, '0')}`,
        );
        setToTime(
          `${e.getHours().toString().padStart(2, '0')}:${e
            .getMinutes()
            .toString()
            .padStart(2, '0')}`,
        );
      }
    }
  }, [currentTimeRange, isCustom]);

  const handlePreset = (r: TimeRange) => {
    updateSearchParams({ start: null, end: null, range: r });
    onTimeRangeChange(r);
    setOpen(false);
  };

  const applyCustom = () => {
    if (!date) return;
    const from = fromTime || '00:00';
    const to = toTime || '23:59';
    const [fh, fm] = from.split(':').map(Number);
    const [th, tm] = to.split(':').map(Number);
    const start = new Date(date);
    start.setHours(fh, fm, 0, 0);
    const end = new Date(date);
    end.setHours(th, tm, 0, 0);
    if (end <= start) {
      end.setDate(end.getDate() + 1);
    }
    const s = start.getTime();
    const e = end.getTime();
    const custom = `${s}-${e}`;
    if (isValidTimeRange(custom)) {
      updateSearchParams({ start: String(s), end: String(e), range: null });
      onTimeRangeChange(custom);
      setOpen(false);
    }
  };

  return (
    <Popover.Root open={open} onOpenChange={setOpen}>
      <Popover.Trigger asChild>
        <button
          disabled={isChanging}
          className="p-1 border border-gray-300 dark:border-gray-600 rounded-md text-sm bg-white dark:bg-gray-800 min-w-[3rem]"
          title={customTooltip}
        >
          {buttonLabel}
        </button>
      </Popover.Trigger>
      <Popover.Content
        side="bottom"
        align="end"
        className="bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-md shadow-lg p-2 space-y-1 z-10"
      >
        {presetRanges.map((r) => (
          <button
            key={r}
            onClick={() => handlePreset(r)}
            className="block w-full text-left px-2 py-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
          >
            {r}
          </button>
        ))}
        <div className="pt-1 border-t border-gray-200 dark:border-gray-700 mt-1 space-y-1">
          <DayPicker
            mode="single"
            selected={date}
            onSelect={(d) => {
              const newDate = d ?? undefined;
              setDate(newDate);
              if (d && !fromTime && !toTime) {
                setFromTime('00:00');
                setToTime('23:59');
              }
            }}
            defaultMonth={date}
          />
          <div className="flex items-center space-x-2">
            <input
              type="time"
              step="900"
              placeholder="hh:mm"
              value={fromTime}
              onChange={(e) => setFromTime(e.target.value)}
              className="border rounded p-1 text-sm bg-white dark:bg-gray-800"
            />
            <span className="text-sm">to</span>
            <input
              type="time"
              step="900"
              placeholder="hh:mm"
              value={toTime}
              onChange={(e) => setToTime(e.target.value)}
              className="border rounded p-1 text-sm bg-white dark:bg-gray-800"
            />
          </div>
          <button
            onClick={applyCustom}
            disabled={isChanging || !date || !fromTime || !toTime}
            className="mt-1 px-2 py-1 text-sm rounded-md bg-gray-200 dark:bg-gray-700 w-full"
          >
            Apply
          </button>
        </div>
      </Popover.Content>
    </Popover.Root>
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
    { label: '1h', value: 60 * 60_000 },
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

