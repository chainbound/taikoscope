import { describe, it, expect } from 'vitest';
import {
  formatDecimal,
  formatSeconds,
  formatInterval,
  formatBatchDuration,
  computeBatchDurationFlags,
  computeIntervalFlags,
  shouldShowMinutes,
  findMetricValue,
  formatSequencerTooltip,
  formatLargeNumber,
  formatWithCommas,
  formatEth,
  parseEthValue,
  bytesToHex,
  loadRefreshRate,
  saveRefreshRate,
  isValidRefreshRate,
  formatHoursMinutes,
} from '../utils';

describe('utils', () => {
  it('formats numbers and durations', () => {
    expect(formatDecimal(1)).toBe('1.0');
    expect(formatDecimal(12.345)).toBe('12.3');

    expect(formatSeconds(30)).toBe('30.0s');
    expect(formatSeconds(150)).toBe('2:30min');
    expect(formatSeconds(7200)).toBe('2:00h');
    expect(formatHoursMinutes(9000)).toBe('2:30');

    expect(formatInterval(30, false, false)).toBe('30 seconds');
    expect(formatInterval(180, false, true)).toBe('3.0 minutes');
    expect(formatInterval(7200, true, false)).toBe('2.0 hours');

    expect(formatBatchDuration(45, false, false)).toBe('45 seconds');
    expect(formatBatchDuration(150, false, true)).toBe('2.5 minutes');
    expect(formatBatchDuration(7200, true, false)).toBe('2.0 hours');
  });

  it('computes batch duration flags', () => {
    const flags = computeBatchDurationFlags([{ value: 30 }, { value: 7200 }]);
    expect(flags.showHours).toBe(true);
    expect(flags.showMinutes).toBe(false);

    const flagsMinutes = computeBatchDurationFlags([
      { value: 150 },
      { value: 100 },
    ]);
    expect(flagsMinutes.showHours).toBe(false);
    expect(flagsMinutes.showMinutes).toBe(true);

    const flagsNone = computeBatchDurationFlags([{ value: 60 }, { value: 80 }]);
    expect(flagsNone.showHours).toBe(false);
    expect(flagsNone.showMinutes).toBe(false);
  });

  it('computes interval flags', () => {
    const flags = computeIntervalFlags(
      [{ timestamp: 1 }, { timestamp: 8_000 }],
      true,
    );
    expect(flags.showHours).toBe(true);
    expect(flags.showMinutes).toBe(false);

    const flagsMinutes = computeIntervalFlags(
      [{ timestamp: 150 }, { timestamp: 100 }],
      true,
    );
    expect(flagsMinutes.showHours).toBe(false);
    expect(flagsMinutes.showMinutes).toBe(false);

    const flagsMinutesAll = computeIntervalFlags(
      [{ timestamp: 150 }, { timestamp: 200 }],
      true,
    );
    expect(flagsMinutesAll.showHours).toBe(false);
    expect(flagsMinutesAll.showMinutes).toBe(true);

    const flagsNone = computeIntervalFlags(
      [{ timestamp: 50 }, { timestamp: 80 }],
      true,
    );
    expect(flagsNone.showHours).toBe(false);
    expect(flagsNone.showMinutes).toBe(false);
  });

  it('determines minute display correctly', () => {
    expect(
      shouldShowMinutes([{ timestamp: 1 }, { timestamp: 200 }], true),
    ).toBe(false);

    expect(
      shouldShowMinutes([{ timestamp: 200 }, { timestamp: 250 }], true),
    ).toBe(true);

    expect(
      shouldShowMinutes([{ timestamp: 1 }, { timestamp: 110 }], true),
    ).toBe(false);
  });

  it('finds metric values', () => {
    const metrics = [
      { title: 'Test', value: '42' },
      { title: 'Another', value: '0' },
    ];
    expect(findMetricValue(metrics, 'test')).toBe('42');
    expect(findMetricValue(metrics, 'TEST')).toBe('42');
    expect(findMetricValue(metrics, 'missing')).toBe('N/A');
  });

  it('formats sequencer tooltip', () => {
    const tooltip = formatSequencerTooltip([{ value: 1 }, { value: 3 }], 1);
    expect(tooltip).toBe('1 blocks (25.00%)');

    const zeroTooltip = formatSequencerTooltip([{ value: 0 }], 0);
    expect(zeroTooltip).toBe('0 blocks (0%)');
  });

  it('formats large numbers', () => {
    expect(formatLargeNumber(1500)).toBe('1.5K');
    expect(formatLargeNumber(15_000_000)).toBe('15M');
    expect(formatLargeNumber(50)).toBe('50');
  });

  it('formats numbers with commas', () => {
    expect(formatWithCommas(1234567)).toBe('1,234,567');
    expect(formatWithCommas(50)).toBe('50');
  });

  it('formats ETH amounts', () => {
    expect(formatEth(42e9)).toBe('42 ETH');
    expect(formatEth(0)).toBe('0 ETH');
    expect(formatEth(0.1)).toBe('0 ETH');
    expect(formatEth(1334501)).toBe('0.001 ETH');
    expect(formatEth(1422636.1)).toBe('0.001 ETH');
    expect(formatEth(1422636.1e9)).toBe('1,422,636 ETH');
    expect(formatEth(187788.9)).toBe('0 ETH');
    expect(formatEth(-0.1)).toBe('0 ETH');
    expect(formatEth(-345678.9)).toBe('0 ETH');
    expect(formatEth(-1.2345e9)).toBe('-1.2 ETH');
    expect(formatEth(0.01e9)).toBe('0.01 ETH');
    expect(formatEth(0.012e9)).toBe('0.012 ETH');
    expect(formatEth(-0.04e9)).toBe('-0.04 ETH');
    expect(formatEth(1e9, 3)).toBe('1 ETH');
  });

  it('parses ETH values', () => {
    expect(parseEthValue('0.6 ETH')).toBe(0.6);
    expect(parseEthValue('N/A')).toBe(0);
  });

  it('parses negative ETH values', () => {
    expect(parseEthValue('-0.5 ETH')).toBe(-0.5);
  });

  it('converts bytes to hex', () => {
    expect(bytesToHex([0, 1, 255])).toBe('0x0001ff');
  });

  it('saves and loads refresh rate', () => {
    const store: Record<string, string> = {};
    globalThis.localStorage = {
      getItem: (k: string) => (k in store ? store[k] : null),
      setItem: (k: string, v: string) => {
        store[k] = v;
      },
      removeItem: () => { },
      clear: () => { },
      key: () => null,
      length: 0,
    } as Storage;

    expect(loadRefreshRate()).toBe(600000);
    saveRefreshRate(600000);
    expect(store.refreshRate).toBe('600000');
    store.refreshRate = '2000';
    expect(loadRefreshRate()).toBe(600000);
  });

  it('validates refresh rate', () => {
    expect(isValidRefreshRate(1000)).toBe(false);
    expect(isValidRefreshRate(-1)).toBe(false);
    expect(isValidRefreshRate(NaN)).toBe(false);
  });
});
