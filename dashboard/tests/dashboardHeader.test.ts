import { describe, it, expect } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { MemoryRouter } from 'react-router-dom';
import { DashboardHeader } from '../components/DashboardHeader';
import { ThemeProvider } from '../contexts/ThemeContext';

describe('DashboardHeader', () => {
  it('renders time range and refresh controls', () => {
    const html = renderToStaticMarkup(
      React.createElement(
        ThemeProvider,
        null,
        React.createElement(
          MemoryRouter,
          null,
          React.createElement(DashboardHeader, {
            timeRange: '1h',
            onTimeRangeChange: () => {},
            refreshRate: 60000,
            onRefreshRateChange: () => {},
            lastRefresh: Date.now(),
            onManualRefresh: () => {},
            sequencers: ['seq1', 'seq2'],
            selectedSequencer: null,
            onSequencerChange: () => {},
          }),
        ),
      ),
    );
    expect(html.includes('Taiko Masaya Testnet')).toBe(true);
    expect(html.includes('1H')).toBe(true);
    expect(html.includes('24H')).toBe(true);
    expect(html.includes('7D')).toBe(true);
    expect(html.includes('Refresh')).toBe(true);
    expect(html.includes('Status')).toBe(true);
    expect(html.includes('Economics')).toBe(true);
  });
});
