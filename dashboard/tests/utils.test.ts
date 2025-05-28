import { describe, it, expect } from 'vitest';
import {
  formatDecimal,
  formatSeconds,
  formatInterval,
  formatBatchDuration,
  computeBatchDurationFlags,
  shouldShowMinutes,
  findMetricValue,
  formatSequencerTooltip,
  formatLargeNumber,
  bytesToHex,
} from '../utils.js';

describe('utils', () => {
  it('formats numbers and durations', () => {
    expect(formatDecimal(1)).toBe('1.00');
    expect(formatDecimal(12.345)).toBe('12.3');

    expect(formatSeconds(30)).toBe('30.0s');
    expect(formatSeconds(150)).toBe('2.5m');
    expect(formatSeconds(7200)).toBe('2h');

    expect(formatInterval(30000, false)).toBe('30 seconds');
    expect(formatInterval(180000, true)).toBe('3.00 minutes');

    expect(formatBatchDuration(45, false, false)).toBe('45 seconds');
    expect(formatBatchDuration(150, false, true)).toBe('2.50 minutes');
    expect(formatBatchDuration(7200, true, false)).toBe('2.00 hours');
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

  it('determines minute display correctly', () => {
    expect(shouldShowMinutes([{ timestamp: 1000 }, { timestamp: 200000 }])).toBe(
      true,
    );

    expect(shouldShowMinutes([{ timestamp: 1000 }, { timestamp: 110000 }])).toBe(
      false,
    );
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

  it('converts bytes to hex', () => {
    expect(bytesToHex([0, 1, 255])).toBe('0x0001ff');
  });
});
