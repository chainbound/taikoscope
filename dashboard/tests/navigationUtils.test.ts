import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  isValidUrl,
  sanitizeUrl,
  createSafeUrl,
  validateSearchParams,
  cleanSearchParams,
  safeNavigate,
} from '../utils/navigationUtils';

// Mock window.location
const mockLocation = {
  origin: 'https://example.com',
  pathname: '/dashboard',
  href: 'https://example.com/dashboard',
};

beforeEach(() => {
  vi.stubGlobal('window', {
    location: mockLocation,
  });
});

describe('navigationUtils', () => {
  describe('isValidUrl', () => {
    it('should return true for valid URLs', () => {
      expect(isValidUrl('https://example.com/path')).toBe(true);
      expect(isValidUrl('/relative/path')).toBe(true);
    });

    it('should return false for invalid URLs', () => {
      expect(isValidUrl('javascript:alert(1)')).toBe(false);
      expect(isValidUrl('data:text/html,<script>alert(1)</script>')).toBe(
        false,
      );
      expect(isValidUrl('')).toBe(false);
      expect(isValidUrl('   ')).toBe(false);
      expect(isValidUrl('https://example.com/evil path')).toBe(false);
      expect(isValidUrl('/../secret')).toBe(false);
    });
  });

  describe('sanitizeUrl', () => {
    it('should preserve same-origin URLs', () => {
      const url = 'https://example.com/dashboard?view=table';
      expect(sanitizeUrl(url)).toBe('/dashboard?view=table');
    });

    it('should reject different-origin URLs', () => {
      const url = 'https://malicious.com/evil';
      expect(sanitizeUrl(url)).toBe('/dashboard');
    });

    it('should handle invalid URLs gracefully', () => {
      expect(sanitizeUrl('javascript:alert(1)')).toBe('/dashboard');
      expect(sanitizeUrl('data:text/html,evil')).toBe('/dashboard');
    });

    it('should remove invalid search parameters', () => {
      const url = 'https://example.com/dashboard?view=table&page=-1&bad=1';
      expect(sanitizeUrl(url)).toBe('/dashboard?view=table');
    });
  });

  describe('createSafeUrl', () => {
    it('should create URL from valid input', () => {
      const result = createSafeUrl('https://example.com/test');
      expect(result.toString()).toBe('https://example.com/test');
    });

    it('should fallback to current location for invalid input', () => {
      const result = createSafeUrl('invalid');
      expect(result.pathname).toBe('/dashboard');
    });
  });

  describe('validateSearchParams', () => {
    it('should validate allowed view parameters', () => {
      const params = new URLSearchParams('view=table');
      expect(validateSearchParams(params)).toBe(true);

      const params2 = new URLSearchParams('view=economics');
      expect(validateSearchParams(params2)).toBe(true);
    });

    it('should reject invalid view parameters', () => {
      const params = new URLSearchParams('view=invalid');
      expect(validateSearchParams(params)).toBe(false);
    });

    it('should validate page parameters', () => {
      const params = new URLSearchParams('page=5');
      expect(validateSearchParams(params)).toBe(true);

      const invalidParams = new URLSearchParams('page=-1');
      expect(validateSearchParams(invalidParams)).toBe(false);

      const invalidParams2 = new URLSearchParams('page=abc');
      expect(validateSearchParams(invalidParams2)).toBe(false);
    });

    it('should validate range parameters', () => {
      const params = new URLSearchParams('range=1h');
      expect(validateSearchParams(params)).toBe(true);

      const invalidParams = new URLSearchParams('range=invalid');
      expect(validateSearchParams(invalidParams)).toBe(false);
    });

    it('should validate sort and filter parameters', () => {
      const params = new URLSearchParams('sort=asc&filter=test');
      expect(validateSearchParams(params)).toBe(true);

      const invalid = new URLSearchParams('sort=up&filter=<bad>');
      expect(validateSearchParams(invalid)).toBe(false);
    });

    it('should validate table parameters', () => {
      const params = new URLSearchParams('table=missed-proposals');
      expect(validateSearchParams(params)).toBe(true);

      const invalid = new URLSearchParams('table=../../etc');
      expect(validateSearchParams(invalid)).toBe(false);
    });
  });

  describe('cleanSearchParams', () => {
    it('should keep only allowed parameters', () => {
      const params = new URLSearchParams(
        'view=table&malicious=script&sequencer=test',
      );
      const cleaned = cleanSearchParams(params);

      expect(cleaned.get('view')).toBe('table');
      expect(cleaned.get('sequencer')).toBe('test');
      expect(cleaned.get('malicious')).toBeNull();
    });

    it('should keep new sort and filter parameters', () => {
      const params = new URLSearchParams('sort=desc&filter=name&bad=1');
      const cleaned = cleanSearchParams(params);

      expect(cleaned.get('sort')).toBe('desc');
      expect(cleaned.get('filter')).toBe('name');
      expect(cleaned.get('bad')).toBeNull();
    });

    it('should trim parameter values', () => {
      const params = new URLSearchParams('view= table &sequencer= test ');
      const cleaned = cleanSearchParams(params);

      expect(cleaned.get('view')).toBe('table');
      expect(cleaned.get('sequencer')).toBe('test');
    });

    it('should handle empty values gracefully', () => {
      const params = new URLSearchParams('view=&sequencer=test');
      const cleaned = cleanSearchParams(params);

      expect(cleaned.get('view')).toBeNull();
      expect(cleaned.get('sequencer')).toBe('test');
    });

    it('should drop invalid parameter values', () => {
      const params = new URLSearchParams('sort=up&filter=<bad>');
      const cleaned = cleanSearchParams(params);

      expect(cleaned.get('sort')).toBeNull();
      expect(cleaned.get('filter')).toBeNull();
    });

    it('should keep valid table parameter', () => {
      const params = new URLSearchParams('table=missed-proposals&bad=1');
      const cleaned = cleanSearchParams(params);

      expect(cleaned.get('table')).toBe('missed-proposals');
      expect(cleaned.get('bad')).toBeNull();
    });
  });

  describe('safeNavigate', () => {
    it('sanitizes URL before navigating', () => {
      const nav = vi.fn();
      safeNavigate(nav, 'https://malicious.com/evil', true);
      expect(nav).toHaveBeenCalledWith('/dashboard', { replace: true });
    });

    it('cleans search params', () => {
      const nav = vi.fn();
      safeNavigate(nav, '/dashboard?page=-1&view=table');
      expect(nav).toHaveBeenCalledWith('/dashboard?view=table', {
        replace: false,
      });
    });

    it('preserves valid page navigation', () => {
      const nav = vi.fn();
      safeNavigate(nav, '/dashboard?view=table&page=2');
      expect(nav).toHaveBeenCalledWith('/dashboard?view=table&page=2', {
        replace: false,
      });
    });
  });
});
