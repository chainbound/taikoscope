import assert from 'assert';
import { createMetrics } from '../helpers.js';
import { groupMetrics, GROUP_ORDER } from '../groupMetrics.js';

const metrics = createMetrics({
  l2Cadence: 60000,
  batchCadence: 30000,
  avgProve: 1200,
  avgVerify: 1000,
  activeGateways: 2,
  currentOperator: '0xabc',
  nextOperator: null,
  l2Reorgs: 1,
  slashings: 0,
  forcedInclusions: 0,
  l2Block: 10,
  l1Block: 20,
});

const grouped = groupMetrics(metrics);

assert.strictEqual(Array.isArray(GROUP_ORDER), true);
assert.ok(GROUP_ORDER.includes('Network Performance'));
assert.ok(grouped['Network Performance'].length > 0);
assert.ok(grouped['Block Information'].length > 0);

const otherGrouped = groupMetrics([{ title: 'x', value: '1' }]);
assert.strictEqual(otherGrouped['Other'][0].title, 'x');

console.log('groupMetrics tests passed.');
