import { describe, it, expect } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { DashboardHeader } from '../components/DashboardHeader';

describe('DashboardHeader', () => {
  it('renders time range and refresh controls', () => {
    const html = renderToStaticMarkup(
      React.createElement(DashboardHeader, {
        timeRange: '1h',
        onTimeRangeChange: () => {},
        refreshRate: 60000,
        onRefreshRateChange: () => {},
        lastRefresh: Date.now(),
        onManualRefresh: () => {},
        isSequencerPage: false,
        onNavigate: () => {},
      }),
    );
    expect(html.includes('Taiko Masaya Testnet')).toBe(true);
    expect(html.includes('1H')).toBe(true);
    expect(html.includes('24H')).toBe(true);
    expect(html.includes('7D')).toBe(true);
    expect(html.includes('Refresh')).toBe(true);
    expect(html.includes('Status')).toBe(true);
    expect(html.includes('Sequencer Details')).toBe(true);
  });
});
