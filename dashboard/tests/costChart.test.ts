import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import * as swr from 'swr';
vi.mock('swr', () => ({ default: vi.fn() }));
import { CostChart } from '../components/CostChart';

const feeData = [
  { block: 1, priority: 1, base: 1, l1Cost: 0 },
];

describe('CostChart', () => {
  it('renders with cost data', () => {
    vi.mocked(swr.default).mockReturnValue({ data: { data: feeData } } as any);

    const html = renderToStaticMarkup(
      React.createElement(CostChart, {
        timeRange: '1h',
        cloudCost: 100,
        proverCost: 100,
      }),
    );

    expect(html).toContain('recharts-responsive-container');
  });
});
