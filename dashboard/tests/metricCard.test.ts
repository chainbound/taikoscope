import { describe, it, expect } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { MetricCard } from '../components/MetricCard';

describe('MetricCard', () => {
  it('renders addresses with special classes', () => {
    const addressValue = '0x1234567890123456789012345678901234567890';
    const htmlAddress = renderToStaticMarkup(
      React.createElement(MetricCard, {
        title: 'Operator',
        value: addressValue,
      }),
    );
    expect(
      htmlAddress.includes(
        'min-w-0 w-full sm:col-span-2 md:col-span-2 lg:col-span-2 xl:col-span-2 2xl:col-span-2',
      ),
    ).toBe(true);
    expect(htmlAddress.includes('text-base break-all')).toBe(true);
  });

  it('renders normal values', () => {
    const htmlNormal = renderToStaticMarkup(
      React.createElement(MetricCard, { title: 'Blocks', value: '42' }),
    );
    expect(htmlNormal.includes('min-w-0 w-full')).toBe(false);
    expect(htmlNormal.includes('text-2xl')).toBe(true);
    expect(htmlNormal.includes('whitespace-nowrap')).toBe(false);
    expect(htmlNormal.includes('42')).toBe(true);
  });

  it('truncates long values with ellipsis', () => {
    const longValue = 'Chainbound extremely long name';
    const html = renderToStaticMarkup(
      React.createElement(MetricCard, { title: 'Current', value: longValue }),
    );
    expect(html.includes('whitespace-nowrap')).toBe(true);
    expect(html.includes('overflow-hidden')).toBe(true);
  });

  it('does not truncate short sequencer names', () => {
    const names = ['Nethermind A', 'Chainbound B', 'Gattaca C'];
    for (const name of names) {
      const html = renderToStaticMarkup(
        React.createElement(MetricCard, { title: 'Current', value: name }),
      );
      expect(html.includes('whitespace-nowrap')).toBe(false);
      expect(html.includes('overflow-hidden')).toBe(false);
    }
  });
});
