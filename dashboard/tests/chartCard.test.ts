import { describe, it, expect } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { ChartCard } from '../components/ChartCard';

describe('ChartCard', () => {
  it('renders title and children', () => {
    const html = renderToStaticMarkup(
      React.createElement(ChartCard, {
        title: 'My Chart',
        children: React.createElement('span', null, 'child'),
      }),
    );
    expect(html.includes('My Chart')).toBe(true);
    expect(html.includes('child')).toBe(true);
    expect(html.includes('aria-label="View table"')).toBe(false);
  });

  it('shows more button when handler provided', () => {
    const html = renderToStaticMarkup(
      React.createElement(ChartCard, {
        title: 'Other Chart',
        onMore: () => {},
        children: React.createElement('div', null, 'data'),
      }),
    );
    expect(html.includes('data')).toBe(true);
    expect(html.includes('aria-label="View table"')).toBe(true);
  });
});
