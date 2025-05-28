import assert from 'assert';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { BatchProcessChart } from '../components/BatchProcessChart.js';

const emptyHtml = renderToStaticMarkup(
  React.createElement(BatchProcessChart, { data: [], lineColor: '#000' }),
);
assert(emptyHtml.includes('No data available'));

const data = [{ name: '1', value: 60, timestamp: 1000 }];
const html = renderToStaticMarkup(
  React.createElement(BatchProcessChart, {
    data,
    lineColor: '#000',
  }),
);
assert(html.includes('recharts-responsive-container'));

console.log('BatchProcessChart tests passed.');
