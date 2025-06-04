import { describe, it, expect } from 'vitest';
import { getErrorMessage } from '../utils/errorHandler';

describe('errorHandler', () => {
  it('returns warning message when bad request detected', () => {
    expect(getErrorMessage(true)).toBe(
      'Invalid parameters provided. Some data may not be available.'
    );
  });

  it('returns empty string when no bad request', () => {
    expect(getErrorMessage(false)).toBe('');
  });
});
