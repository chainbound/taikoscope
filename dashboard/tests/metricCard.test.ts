import assert from 'assert';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { MetricCard } from '../components/MetricCard.js';

const addressValue = '0x1234567890123456789012345678901234567890';
const htmlAddress = renderToStaticMarkup(
  React.createElement(MetricCard, { title: 'Operator', value: addressValue }),
);
assert(htmlAddress.includes('min-w-0 w-full sm:col-span-2'));
assert(htmlAddress.includes('text-lg whitespace-nowrap'));

const htmlNormal = renderToStaticMarkup(
  React.createElement(MetricCard, { title: 'Blocks', value: '42' }),
);
assert(!htmlNormal.includes('min-w-0 w-full'));
assert(htmlNormal.includes('text-3xl break-all'));
assert(htmlNormal.includes('42'));

console.log('MetricCard tests passed.');
