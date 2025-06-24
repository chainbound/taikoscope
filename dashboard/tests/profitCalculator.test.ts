import { describe, it, expect } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { ProfitCalculator } from '../components/ProfitCalculator';

describe('ProfitCalculator', () => {
  it('renders hardware cost inputs', () => {
    const html = renderToStaticMarkup(
      React.createElement(ProfitCalculator, {
        timeRange: '1h',
        cloudCost: 100,
        proverCost: 200,
        onCloudCostChange: () => { },
        onProverCostChange: () => { },
      }),
    );
    expect(html.includes('Hardware Costs')).toBe(true);
    expect(html.includes('Monthly Cloud Cost')).toBe(true);
    expect(html.includes('Prover Cost')).toBe(true);
  });

  it('sets correct input values', () => {
    const html = renderToStaticMarkup(
      React.createElement(ProfitCalculator, {
        timeRange: '1h',
        cloudCost: 150,
        proverCost: 250,
        onCloudCostChange: () => { },
        onProverCostChange: () => { },
      }),
    );
    expect(html.includes('value="150"')).toBe(true);
    expect(html.includes('value="250"')).toBe(true);
  });

  it('sets min attribute to 0 for both inputs', () => {
    const html = renderToStaticMarkup(
      React.createElement(ProfitCalculator, {
        timeRange: '1h',
        cloudCost: 0,
        proverCost: 0,
        onCloudCostChange: () => { },
        onProverCostChange: () => { },
      }),
    );
    const matches = html.match(/min="0"/g) ?? [];
    expect(matches.length).toBe(2);
  });
});
