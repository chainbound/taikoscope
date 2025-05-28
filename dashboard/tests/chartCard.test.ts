import assert from 'assert';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { ChartCard } from '../components/ChartCard.js';

const htmlNoMore = renderToStaticMarkup(
  React.createElement(ChartCard, { title: 'Chart', children: 'content' }),
);
assert(!htmlNoMore.includes('aria-label="View table"'));
assert(htmlNoMore.includes('Chart'));
assert(htmlNoMore.includes('content'));

const htmlMore = renderToStaticMarkup(
  React.createElement(ChartCard, {
    title: 'Chart',
    onMore: () => {},
    children: 'body',
  }),
);
assert(htmlMore.includes('aria-label="View table"'));

console.log('ChartCard tests passed.');
