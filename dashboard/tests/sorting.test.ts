import { describe, it, expect } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import VirtualizedTable from '../components/VirtualizedTable';

const columns = [{ key: 'val', label: 'Val', sortable: true }];

describe('VirtualizedTable sorting', () => {
  it('sorts numeric rows descending', () => {
    const data = [{ val: '1' }, { val: '3' }, { val: '2' }];
    const html = renderToStaticMarkup(
      React.createElement(VirtualizedTable, {
        columns,
        data,
        sortBy: 'val',
        sortDirection: 'desc',
        height: 160,
        rowHeight: 40,
      }),
    );
    const first = html.indexOf('>3<');
    const second = html.indexOf('>2<');
    const third = html.indexOf('>1<');
    expect(first).toBeGreaterThan(-1);
    expect(second).toBeGreaterThan(first);
    expect(third).toBeGreaterThan(second);
  });

  it('sorts string rows descending', () => {
    const data = [{ val: 'a' }, { val: 'c' }, { val: 'b' }];
    const html = renderToStaticMarkup(
      React.createElement(VirtualizedTable, {
        columns,
        data,
        sortBy: 'val',
        sortDirection: 'desc',
        height: 160,
        rowHeight: 40,
      }),
    );
    const first = html.indexOf('>c<');
    const second = html.indexOf('>b<');
    const third = html.indexOf('>a<');
    expect(first).toBeGreaterThan(-1);
    expect(second).toBeGreaterThan(first);
    expect(third).toBeGreaterThan(second);
  });
});
