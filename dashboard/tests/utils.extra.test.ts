import { describe, it, expect } from 'vitest';
import { formatDecimal, formatSeconds } from '../utils.js';

describe('extra utils', () => {
  it('formats decimals and seconds', () => {
    expect(formatDecimal(-5.678)).toBe('-5.68');
    expect(formatSeconds(0)).toBe('0.00s');
    expect(formatSeconds(119)).toBe('119.0s');
    expect(formatSeconds(120)).toBe('2m');
  });
});
