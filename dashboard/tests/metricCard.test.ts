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
    expect(htmlAddress.includes('text-base sm:text-lg break-all')).toBe(true);
  });

  it('renders normal values', () => {
    const htmlNormal = renderToStaticMarkup(
      React.createElement(MetricCard, { title: 'Blocks', value: '42' }),
    );
    expect(htmlNormal.includes('min-w-0 w-full')).toBe(false);
    expect(htmlNormal.includes('text-3xl whitespace-nowrap overflow-hidden text-ellipsis')).toBe(true);
    expect(htmlNormal.includes('42')).toBe(true);
  });
});
