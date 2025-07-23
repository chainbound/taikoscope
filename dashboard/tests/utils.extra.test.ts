import { describe, it, expect } from 'vitest';
import { formatDecimal, formatSeconds, formatSecondsWithDecimal, formatHoursMinutes } from '../utils';

describe('extra utils', () => {
  describe('formatDecimal', () => {
    it.each([
      [-5.678, '-5.7'],
      [0, '0.00'],
      [12.345, '12.3'],
      [0.01, '0.01'],
      [-0.04, '-0.04'],
    ])('formats %p to %p', (input, expected) => {
      expect(formatDecimal(input)).toBe(expected);
    });
  });

  describe('formatSeconds', () => {
    it.each([
      [0, '0s'],
      [119, '119s'],
      [120, '2:00min'],
      [7200, '2:00h'],
    ])('formats %p seconds to %p', (input, expected) => {
      expect(formatSeconds(input)).toBe(expected);
    });
  });

  describe('formatSecondsWithDecimal', () => {
    it.each([
      [0, '0.0s'],
      [3.1, '3.1s'],
      [2.8, '2.8s'],
      [119.7, '119.7s'],
      [120, '2:00min'],
      [7200, '2:00h'],
    ])('formats %p seconds to %p', (input, expected) => {
      expect(formatSecondsWithDecimal(input)).toBe(expected);
    });
  });

  describe('formatHoursMinutes', () => {
    it.each([
      [3600, '1:00'],
      [3661, '1:01'],
      [9000, '2:30'],
    ])('formats %p seconds to %p', (input, expected) => {
      expect(formatHoursMinutes(input)).toBe(expected);
    });
  });
});
