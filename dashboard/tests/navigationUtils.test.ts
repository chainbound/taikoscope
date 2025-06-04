import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  isValidUrl,
  sanitizeUrl,
  createSafeUrl,
  validateSearchParams,
  cleanSearchParams,
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
    });

    it('should reject unsupported protocols', () => {
      expect(isValidUrl('ftp://example.com/file')).toBe(false);
    });
  });

  describe('sanitizeUrl', () => {
    it('should preserve same-origin URLs', () => {
      const url = 'https://example.com/dashboard?view=table';
      expect(sanitizeUrl(url)).toBe(url);
    });

    it('should reject different-origin URLs', () => {
      const url = 'https://malicious.com/evil';
      expect(sanitizeUrl(url)).toBe('/dashboard');
    });

    it('should handle invalid URLs gracefully', () => {
      expect(sanitizeUrl('javascript:alert(1)')).toBe('/dashboard');
      expect(sanitizeUrl('data:text/html,evil')).toBe('/dashboard');
    });

    it('should resolve relative URLs against the current origin', () => {
      const result = sanitizeUrl('/relative/page?foo=bar');
      expect(result).toBe('https://example.com/relative/page?foo=bar');
    });

    it('should reject URL objects from a different origin', () => {
      const url = new URL('https://evil.com');
      expect(sanitizeUrl(url)).toBe('/dashboard');
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

    it('should default to current location when no input is provided', () => {
      const result = createSafeUrl();
      expect(result.toString()).toBe('https://example.com/dashboard');
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

    it('should handle edge cases for page and range parameters', () => {
      const zeroPage = new URLSearchParams('page=0');
      expect(validateSearchParams(zeroPage)).toBe(true);

      const emptyValues = new URLSearchParams('view=&page=&range=');
      expect(validateSearchParams(emptyValues)).toBe(true);

      const range24h = new URLSearchParams('range=24h');
      expect(validateSearchParams(range24h)).toBe(true);

      const range7d = new URLSearchParams('range=7d');
      expect(validateSearchParams(range7d)).toBe(true);
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

    it('should keep the last value when parameters repeat', () => {
      const params = new URLSearchParams(
        'view=table&view=economics&sequencer=a',
      );
      const cleaned = cleanSearchParams(params);

      expect(cleaned.get('view')).toBe('economics');
      expect(cleaned.get('sequencer')).toBe('a');
    });
  });
});
