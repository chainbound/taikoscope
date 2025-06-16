import { describe, it, expect } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import {
  blockLink,
  addressLink,
  formatTime,
  computeIntervalFlags,
  shouldShowMinutes,
  isValidRefreshRate,
  loadRefreshRate,
} from '../utils';

// Test additional utility functions

describe('utils additional', () => {
  it('creates a block link element', () => {
    const el = blockLink(42);
    const html = renderToStaticMarkup(el);
    expect(html).toContain('href');
    const props = (el as any).props;
    expect(props.href.endsWith('/block/42')).toBe(true);
    expect(props.target).toBe('_blank');
    expect(props.rel).toBe('noopener noreferrer');
    expect(props.children).toBe('42');
  });

  it('creates an address link element', () => {
    const el = addressLink('0xabc', 'foo');
    const html = renderToStaticMarkup(el);
    expect(html).toContain('href');
    const props = (el as any).props;
    expect(props.href.endsWith('/address/0xabc')).toBe(true);
    expect(props.children).toBe('foo');
    expect(props.target).toBe('_blank');
    expect(props.rel).toBe('noopener noreferrer');
  });

  it('formats time in UTC', () => {
    const ms = Date.UTC(1970, 0, 1, 12, 34, 56);
    expect(formatTime(ms)).toBe('12:34:56');
  });

  it('computes interval flags using milliseconds', () => {
    const flags = computeIntervalFlags([
      { timestamp: 7200_000 },
      { timestamp: 1000 },
    ]);
    expect(flags.showHours).toBe(true);
    expect(flags.showMinutes).toBe(false);
  });

  it('should show minutes by default using ms', () => {
    const show = shouldShowMinutes([
      { timestamp: 200_000 },
      { timestamp: 250_000 },
    ]);
    expect(show).toBe(true);
  });

  it('validates refresh rate positively', () => {
    expect(isValidRefreshRate(60000)).toBe(true);
  });

  it('loads refresh rate when localStorage is missing', () => {
    const prev = (globalThis as any).localStorage;
    // Ensure localStorage is undefined
    delete (globalThis as any).localStorage;
    expect(loadRefreshRate()).toBe(600000);
    if (prev !== undefined) (globalThis as any).localStorage = prev;
  });
});
