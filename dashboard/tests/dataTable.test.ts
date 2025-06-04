import { describe, it, expect } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { DataTable } from '../components/DataTable';

describe('DataTable', () => {
  it('renders table rows and columns', () => {
    const html = renderToStaticMarkup(
      React.createElement(DataTable, {
        title: 'My Table',
        description: 'info',
        columns: [
          { key: 'a', label: 'A' },
          { key: 'b', label: 'B' },
        ],
        rows: [
          { a: '1', b: '2' },
          { a: '3', b: '4' },
        ],
        onBack: () => {},
      }),
    );
    expect(html.includes('A')).toBe(true);
    expect(html.includes('B')).toBe(true);
    expect(html.includes('1')).toBe(true);
    expect(html.includes('4')).toBe(true);
    expect(html.includes('info')).toBe(true);
  });

  it('renders extra action and extra table', () => {
    const html = renderToStaticMarkup(
      React.createElement(DataTable, {
        title: 'Main',
        columns: [{ key: 'x', label: 'X' }],
        rows: [{ x: '10' }],
        onBack: () => {},
        extraAction: { label: 'More', onClick: () => {} },
        extraTable: {
          title: 'Extra',
          columns: [{ key: 'y', label: 'Y' }],
          rows: [{ y: '20' }],
        },
      }),
    );
    expect(html.includes('More')).toBe(true);
    expect(html.includes('Extra')).toBe(true);
    expect(html.includes('20')).toBe(true);
  });

  it('renders time range selector', () => {
    const html = renderToStaticMarkup(
      React.createElement(DataTable, {
        title: 'Range',
        columns: [{ key: 'v', label: 'V' }],
        rows: [{ v: 1 }],
        onBack: () => {},
        timeRange: '1h',
        onTimeRangeChange: () => {},
      }),
    );
    expect(html.includes('1H')).toBe(true);
    expect(html.includes('24H')).toBe(true);
    expect(html.includes('7D')).toBe(true);
  });

  it('renders refresh rate input', () => {
    const html = renderToStaticMarkup(
      React.createElement(DataTable, {
        title: 'Refresh',
        columns: [{ key: 'v', label: 'V' }],
        rows: [{ v: 1 }],
        onBack: () => {},
        refreshRate: 60000,
        onRefreshRateChange: () => {},
      }),
    );
    expect(html.includes('Refresh')).toBe(true);
  });

  it('renders refresh countdown when props provided', () => {
    const html = renderToStaticMarkup(
      React.createElement(DataTable, {
        title: 'Countdown',
        columns: [{ key: 'v', label: 'V' }],
        rows: [{ v: 1 }],
        onBack: () => {},
        refreshRate: 60000,
        onRefreshRateChange: () => {},
        lastRefresh: Date.now(),
        onManualRefresh: () => {},
      }),
    );
    expect(html.includes('Refresh now')).toBe(true);
  });

  it('renders sequencer selector', () => {
    const html = renderToStaticMarkup(
      React.createElement(DataTable, {
        title: 'Seq',
        columns: [{ key: 'v', label: 'V' }],
        rows: [{ v: 1 }],
        onBack: () => {},
        sequencers: ['b', 'a'],
        selectedSequencer: null,
        onSequencerChange: () => {},
      }),
    );
    const options = Array.from(html.matchAll(/<option[^>]*>(.*?)<\/option>/g)).map(
      (m) => m[1],
    );
    expect(options).toEqual(['All Sequencers', 'a', 'b']);
  });

  it('paginates rows when more than 50 items', () => {
    const rows = Array.from({ length: 55 }, (_, i) => ({ a: String(i) }));
    const html = renderToStaticMarkup(
      React.createElement(DataTable, {
        title: 'Paged',
        columns: [{ key: 'a', label: 'A' }],
        rows,
        onBack: () => {},
      }),
    );
    expect(html.includes('49')).toBe(true);
    expect(html.includes('54')).toBe(false);
    expect(html.includes('Next')).toBe(true);
  });

  it('supports custom rows per page', () => {
    const rows = Array.from({ length: 10 }, (_, i) => ({ a: String(i) }));
    const html = renderToStaticMarkup(
      React.createElement(DataTable, {
        title: 'Custom',
        columns: [{ key: 'a', label: 'A' }],
        rows,
        onBack: () => {},
        rowsPerPage: 5,
      }),
    );
    // Check for actual cell content, not just any occurrence of the character
    expect(html.includes('>4<')).toBe(true);
    expect(html.includes('>5<')).toBe(false);
    expect(html.includes('Next')).toBe(true);
  });

  it('disables pagination buttons when not clickable', () => {
    const rows = [{ a: '1' }, { a: '2' }];
    const html = renderToStaticMarkup(
      React.createElement(DataTable, {
        title: 'Disabled',
        columns: [{ key: 'a', label: 'A' }],
        rows,
        onBack: () => {},
      }),
    );
    expect(html.includes('disabled')).toBe(true);
  });
});
