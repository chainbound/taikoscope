import assert from 'assert';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { SequencerPieChart } from '../components/SequencerPieChart.js';

const emptyHtml = renderToStaticMarkup(
  React.createElement(SequencerPieChart, { data: [] }),
);
assert(emptyHtml.includes('No data available'));

const data = [
  { name: 'Nethermind', value: 2 },
  { name: 'Other', value: 1 },
];
const html = renderToStaticMarkup(
  React.createElement(SequencerPieChart, { data }),
);
assert(html.includes('recharts-responsive-container'));
assert(!html.includes('No data available'));

console.log('SequencerPieChart tests passed.');
