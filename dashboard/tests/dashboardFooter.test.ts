import { describe, it, expect } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { DashboardFooter } from '../components/DashboardFooter';

describe('DashboardFooter', () => {
  it('renders block numbers', () => {
    const html = renderToStaticMarkup(
      React.createElement(DashboardFooter, {
        l2HeadBlock: '409,253',
        l1HeadBlock: '3,951,872',
      })
    );
    expect(html.includes('L2 Head Block')).toBe(true);
    expect(html.includes('409,253')).toBe(true);
    expect(html.includes('L1 Head Block')).toBe(true);
    expect(html.includes('3,951,872')).toBe(true);
  });
});

