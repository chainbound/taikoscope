import assert from 'assert';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { BlockTimeChart } from '../components/BlockTimeChart.js';

const emptyHtml = renderToStaticMarkup(
  React.createElement(BlockTimeChart, { data: [], lineColor: '#000' }),
);
assert(emptyHtml.includes('No data available'));

const data = [
  { value: 1, timestamp: 1000 },
  { value: 2, timestamp: 130000 },
];
const chartHtml = renderToStaticMarkup(
  React.createElement(BlockTimeChart, { data, lineColor: '#000' }),
);
assert(chartHtml.includes('recharts-responsive-container'));

console.log('BlockTimeChart tests passed.');
