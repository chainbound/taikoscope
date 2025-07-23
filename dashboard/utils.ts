import React from 'react';
import { TAIKO_PINK } from './theme';

const rawNetworkName =
  import.meta.env.VITE_NETWORK_NAME ?? import.meta.env.NETWORK_NAME;

export const TAIKOSCAN_BASE =
  import.meta.env.VITE_TAIKOSCAN_BASE ??
  import.meta.env.TAIKOSCAN_BASE ??
  (rawNetworkName?.toLowerCase() === 'mainnet'
    ? 'https://taikoscan.io'
    : rawNetworkName?.toLowerCase() === 'hekla'
      ? 'https://hekla.taikoscan.io'
      : 'https://hekla.taikoscan.io');

export const ETHERSCAN_BASE =
  import.meta.env.VITE_ETHERSCAN_BASE ??
  import.meta.env.ETHERSCAN_BASE ??
  'https://holesky.etherscan.io';

export const blockLink = (
  block: number,
  text?: string | number,
): React.ReactElement =>
  React.createElement(
    'a',
    {
      href: `${TAIKOSCAN_BASE}/block/${block}`,
      target: '_blank',
      rel: 'noopener noreferrer',
      className: 'font-semibold hover:underline',
      style: { color: TAIKO_PINK },
    },
    text ?? block.toLocaleString(),
  );

export const l1BlockLink = (
  block: number,
  text?: string | number,
): React.ReactElement =>
  React.createElement(
    'a',
    {
      href: `${ETHERSCAN_BASE}/block/${block}`,
      target: '_blank',
      rel: 'noopener noreferrer',
      className: 'font-semibold hover:underline',
      style: { color: TAIKO_PINK },
    },
    text ?? block.toLocaleString(),
  );

export const l1TxLink = (
  txHash: string,
  text?: string | number,
): React.ReactElement =>
  React.createElement(
    'a',
    {
      href: `${ETHERSCAN_BASE}/tx/${txHash}`,
      target: '_blank',
      rel: 'noopener noreferrer',
      className: 'font-semibold hover:underline',
      style: { color: TAIKO_PINK },
    },
    text ?? txHash,
  );

export const addressLink = (
  address: string,
  text?: string,
): React.ReactElement =>
  React.createElement(
    'a',
    {
      href: `${TAIKOSCAN_BASE}/address/${address}`,
      target: '_blank',
      rel: 'noopener noreferrer',
      className: 'font-semibold hover:underline',
      style: { color: TAIKO_PINK },
    },
    text ?? address,
  );

export const formatDecimal = (
  value: number,
  decimalsOverride?: number,
): string => {
  if (value === 0) {
    const decimals = decimalsOverride ?? 2;
    return `0.${'0'.repeat(decimals)}`;
  }

  const decimals = decimalsOverride ?? (Math.abs(value) >= 1 ? 1 : 3);
  const factor = 10 ** decimals;
  const rounded = Math.round(value * factor) / factor;
  let result = rounded.toFixed(decimals);

  if (decimalsOverride === undefined && Math.abs(value) < 1) {
    result = result.replace(/0+$/, '');
  }

  return result;
};

export const formatMinutesSeconds = (seconds: number): string => {
  const secs = Math.floor(seconds);
  const mins = Math.floor(secs / 60);
  const rem = secs % 60;
  return `${mins}:${rem.toString().padStart(2, '0')}min`;
};

export const formatSeconds = (seconds: number): string => {
  if (seconds >= 120 * 60) {
    return formatHoursMinutes(seconds) + 'h';
  }
  if (seconds >= 120) {
    return formatMinutesSeconds(seconds);
  }
  return `${Math.round(seconds)}s`;
};

export const formatHoursMinutes = (seconds: number): string => {
  const secs = Math.round(seconds);
  const hrs = Math.floor(secs / 3600);
  const mins = Math.floor((secs % 3600) / 60);
  return `${hrs}:${mins.toString().padStart(2, '0')}`;
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

export const formatWithCommas = (value: number): string =>
  value.toLocaleString();

export const formatEth = (wei: number, decimals?: number): string => {
  if (!Number.isFinite(wei) || Number.isNaN(wei)) {
    return '0 ETH';
  }
  const eth = wei / 1e9;
  if (Math.abs(eth) >= 1000) {
    return `${Math.trunc(eth).toLocaleString()} ETH`;
  }
  const ethFormatted = formatDecimal(eth, decimals);
  const ethTrimmed = String(Number(ethFormatted));
  return `${ethTrimmed} ETH`;
};

export const parseEthValue = (value: string): number => {
  const sanitized = value.replace(/[^0-9.-]/g, '').replace(/(?!^)-/g, '');
  const amount = parseFloat(sanitized);
  return Number.isFinite(amount) ? amount : 0;
};

export const formatUsd = (value: number): string => {
  const abs = Math.abs(value);
  if (abs >= 1000) {
    return Math.trunc(value).toLocaleString();
  }
  return value.toLocaleString(undefined, {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
};

export const formatTime = (ms: number): string =>
  new Date(ms).toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
    timeZone: 'UTC',
  });

export const formatDateTime = (ms: number): string =>
  new Date(ms).toLocaleString('en-GB');

export const formatInterval = (
  seconds: number,
  showHours: boolean,
  showMinutes: boolean,
): string => {
  return showHours
    ? `${formatDecimal(seconds / 3600)} hours`
    : showMinutes
      ? `${formatDecimal(seconds / 60)} minutes`
      : `${Math.round(seconds)} seconds`;
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

export const computeIntervalFlags = (
  data: { timestamp: number }[],
  seconds = false,
) => {
  const toSeconds = (v: number) => (seconds ? v : v / 1000);
  const showHours = data.some((d) => toSeconds(d.timestamp) >= 120 * 60);
  const showMinutes =
    !showHours && data.every((d) => toSeconds(d.timestamp) >= 120);
  return { showHours, showMinutes };
};

export const shouldShowMinutes = (
  data: { timestamp: number }[],
  seconds = false,
) => computeIntervalFlags(data, seconds).showMinutes;

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
  if (typeof localStorage === 'undefined') return 0;
  try {
    const stored = localStorage.getItem('refreshRate');
    const value = stored ? parseInt(stored, 10) : NaN;
    if (!Number.isFinite(value) || value < 0 || (value > 0 && value < 300_000)) {
      localStorage.removeItem('refreshRate');
      return 0;
    }
    return value;
  } catch (err) {
    console.error('Failed to access localStorage:', err);
    return 0;
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
  Number.isFinite(rate) && (rate === 0 || rate >= 300_000);
