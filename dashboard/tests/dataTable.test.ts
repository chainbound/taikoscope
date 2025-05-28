import { describe, it, expect } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { DataTable } from '../components/DataTable.js';

describe('DataTable', () => {
  it('renders table rows and columns', () => {
    const html = renderToStaticMarkup(
      React.createElement(DataTable, {
        title: 'My Table',
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
});
