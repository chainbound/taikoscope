import React from 'react';

export const TAIKOSCAN_BASE =
  ((import.meta as any).env.VITE_TAIKOSCAN_BASE as string | undefined) ??
  ((import.meta as any).env.TAIKOSCAN_BASE as string | undefined) ??
  'https://taikoscan.io';

export const blockLink = (block: number): React.ReactElement =>
  React.createElement(
    'a',
    {
      href: `${TAIKOSCAN_BASE}/block/${block}`,
      target: '_blank',
      rel: 'noopener noreferrer',
      className: 'hover:underline',
    },
    String(block),
  );

export const formatDecimal = (value: number): string => {
  const decimals = Math.abs(value) >= 10 ? 1 : 2;
  return value.toFixed(decimals);
};

export const formatSeconds = (seconds: number): string => {
  if (seconds >= 120 * 60) {
    return `${Number(formatDecimal(seconds / 3600))}h`;
  }
  if (seconds >= 120) {
    return `${Number(formatDecimal(seconds / 60))}m`;
  }
  return `${formatDecimal(seconds)}s`;
};

export const formatLargeNumber = (value: number): string => {
  if (Math.abs(value) >= 1_000_000) {
    return `${Number(formatDecimal(value / 1_000_000))}M`;
  }
  if (Math.abs(value) >= 1_000) {
    return `${Number(formatDecimal(value / 1_000))}K`;
  }
  return value.toLocaleString();
};

export const formatEth = (wei: number): string =>
  `${formatDecimal(wei / 1e18)} ETH`;

export const formatTime = (ms: number): string =>
  new Date(ms).toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
    timeZone: 'UTC',
  });

export const formatInterval = (ms: number, showMinutes: boolean): string => {
  return showMinutes
    ? `${formatDecimal(ms / 60000)} minutes`
    : `${Number(formatDecimal(ms / 1000))} seconds`;
};

export const formatBatchDuration = (
  value: number,
  showHours: boolean,
  showMinutes: boolean,
): string => {
  return showHours
    ? `${formatDecimal(value / 3600)} hours`
    : showMinutes
      ? `${formatDecimal(value / 60)} minutes`
      : `${Math.round(value)} seconds`;
};

export const computeBatchDurationFlags = (data: { value: number }[]) => {
  const showHours = data.some((d) => d.value >= 120 * 60);
  const showMinutes = !showHours && data.some((d) => d.value >= 120);
  return { showHours, showMinutes };
};

export const shouldShowMinutes = (data: { timestamp: number }[]) => {
  return data.some((d) => d.timestamp >= 120000);
};

export const findMetricValue = (
  metrics: { title: string | unknown; value: string }[],
  titlePart: string,
) => {
  const metric = metrics.find((m) => {
    const titleStr = typeof m.title === 'string' ? m.title : '';
    return titleStr.toLowerCase().includes(titlePart.toLowerCase());
  });
  return metric ? metric.value : 'N/A';
};

export const formatSequencerTooltip = (
  data: { value: number }[],
  value: number,
) => {
  const total = data.reduce((acc, curr) => acc + curr.value, 0);
  const percentage = total > 0 ? ((value / total) * 100).toFixed(2) : '0';
  return `${value} blocks (${percentage}%)`;
};

export const bytesToHex = (bytes: number[]): string =>
  `0x${bytes.map((b) => b.toString(16).padStart(2, '0')).join('')}`;

export const loadRefreshRate = (): number => {
  if (typeof localStorage === 'undefined') return 60000;
  try {
    const stored = localStorage.getItem('refreshRate');
    const value = stored ? parseInt(stored, 10) : NaN;
    if (!Number.isFinite(value) || value < 60000) {
      localStorage.removeItem('refreshRate');
      return 60000;
    }
    return value;
  } catch (err) {
    console.error('Failed to access localStorage:', err);
    return 60000;
  }
};

export const saveRefreshRate = (rate: number): void => {
  if (typeof localStorage === 'undefined') return;
  try {
    localStorage.setItem('refreshRate', String(rate));
  } catch (err) {
    console.error('Failed to save refresh rate:', err);
  }
};

export const isValidRefreshRate = (rate: number): boolean =>
  Number.isFinite(rate) && rate >= 60000;
