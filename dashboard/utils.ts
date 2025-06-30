import React from 'react';
import { TAIKO_PINK } from './theme';

const rawNetworkName =
  import.meta.env.VITE_NETWORK_NAME ??
  import.meta.env.NETWORK_NAME;

export const TAIKOSCAN_BASE =
  import.meta.env.VITE_TAIKOSCAN_BASE ??
  import.meta.env.TAIKOSCAN_BASE ??
  (rawNetworkName?.toLowerCase() === 'hekla'
    ? 'https://hekla.taikoscan.io'
    : 'https://cb-blockscout-masaya.vercel.app/blocks');

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

export const l1BlockLink = (block: number, text?: string | number): React.ReactElement =>
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

export const formatDecimal = (value: number): string => {
  if (value === 0) {
    return '0.00';
  }

  const abs = Math.abs(value);

  if (abs >= 1) {
    return value.toLocaleString('en', {
      minimumFractionDigits: 1,
      maximumFractionDigits: 1,
    });
  }

  let result = abs.toLocaleString('en', {
    useGrouping: false,
    maximumFractionDigits: 20,
  });

  const [, decimalPart = ''] = result.split('.');
  if (decimalPart.startsWith('0')) {
    const leadingZeros = decimalPart.match(/^0*/)?.[0].length ?? 0;
    if (leadingZeros >= 2 && decimalPart.length < leadingZeros + 2) {
      result = `${result}${'0'.repeat(leadingZeros + 2 - decimalPart.length)}`;
    }
  }

  return value < 0 ? `-${result}` : result;
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

export const formatWithCommas = (value: number): string =>
  value.toLocaleString();

export const formatEth = (wei: number): string => {
  if (Math.abs(wei) < 1e9) {
    return `${wei.toLocaleString()} wei`;
  }
  const eth = wei / 1e18;
  if (Math.abs(eth) >= 1000) {
    return `${Math.trunc(eth).toLocaleString()} ETH`;
  }
  const ethFormatted = formatDecimal(eth);
  if (wei !== 0 && Math.abs(eth) < 0.005) {
    const gwei = wei / 1e9;
    if (Math.abs(gwei) >= 1000) {
      return `${Math.trunc(gwei).toLocaleString()} Gwei`;
    }
    const gweiFormatted = Number.isInteger(gwei)
      ? gwei.toLocaleString()
      : formatDecimal(gwei);
    return `${gweiFormatted} Gwei`;
  }
  return `${ethFormatted} ETH`;
};

export const parseEthValue = (value: string): number => {
  const sanitized = value
    .replace(/[^0-9.-]/g, '')
    .replace(/(?!^)-/g, '');
  const amount = parseFloat(sanitized);
  if (!Number.isFinite(amount)) return 0;
  return /gwei/i.test(value) ? amount / 1e9 : amount;
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
      : `${Number(formatDecimal(seconds))} seconds`;
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
  if (typeof localStorage === 'undefined') return 600000;
  try {
    const stored = localStorage.getItem('refreshRate');
    const value = stored ? parseInt(stored, 10) : NaN;
    if (!Number.isFinite(value) || value < 60000) {
      localStorage.removeItem('refreshRate');
      return 600000;
    }
    return value;
  } catch (err) {
    console.error('Failed to access localStorage:', err);
    return 600000;
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
