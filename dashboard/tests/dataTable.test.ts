import assert from 'assert';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { DataTable } from '../components/DataTable.js';

const columns = [
  { key: 'a', label: 'A' },
  { key: 'b', label: 'B' },
];
const rows = [{ a: '1', b: '2' }];

const html = renderToStaticMarkup(
  React.createElement(DataTable, {
    title: 'Numbers',
    columns,
    rows,
    onBack: () => {},
  }),
);
assert(html.includes('Numbers'));
assert(html.includes('A'));
assert(html.includes('B'));
assert(html.includes('1'));
assert(html.includes('2'));

console.log('DataTable tests passed.');
