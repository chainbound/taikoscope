import { describe, it, expect } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { MemoryRouter } from 'react-router-dom';
import { DashboardHeader } from '../components/DashboardHeader';
import { ThemeProvider } from '../contexts/ThemeContext';
import { ErrorProvider } from '../hooks/useErrorHandler';

describe('DashboardHeader', () => {
  it('renders time range and refresh controls', () => {
    const html = renderToStaticMarkup(
      React.createElement(
        ThemeProvider,
        null,
        React.createElement(
          ErrorProvider,
          null,
          React.createElement(
            MemoryRouter,
            { initialEntries: ['/?view=performance'] },
            React.createElement(DashboardHeader, {
              timeRange: '1h',
              onTimeRangeChange: () => { },
              refreshRate: 3_600_000,
              onRefreshRateChange: () => { },
              lastRefresh: Date.now(),
              onManualRefresh: () => { },
            }),
          ),
        ),
      ),
    );
    expect(html.includes('Taikoscope Hekla')).toBe(true);
    expect(html.includes('1h')).toBe(true);
    expect(html.includes('Refresh')).toBe(true);
    expect(html.includes('Status')).toBe(true);
    expect(html.includes('Performance')).toBe(true);
    expect(html.includes('Economics')).toBe(true);
    expect(html.includes('Health')).toBe(true);
  });

  it('hides sequencer selector in economics view', () => {
    const html = renderToStaticMarkup(
      React.createElement(
        ThemeProvider,
        null,
        React.createElement(
          ErrorProvider,
          null,
          React.createElement(
            MemoryRouter,
            { initialEntries: ['/?view=economics'] },
            React.createElement(DashboardHeader, {
              timeRange: '1h',
              onTimeRangeChange: () => { },
              refreshRate: 3_600_000,
              onRefreshRateChange: () => { },
              lastRefresh: Date.now(),
              onManualRefresh: () => { },
            }),
          ),
        ),
      ),
    );
    expect(html.includes('Taikoscope Hekla')).toBe(true);
  });
});
