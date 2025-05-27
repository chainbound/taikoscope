import assert from 'assert';
import { createMetrics, hasBadRequest } from '../helpers.js';

const metrics = createMetrics({
  l2Cadence: 60000,
  batchCadence: null,
  avgProve: 1200,
  avgVerify: 0,
  activeGateways: 2,
  currentOperator: '0xabc',
  nextOperator: null,
  l2Reorgs: 1,
  slashings: null,
  forcedInclusions: 0,
  l2Block: 100,
  l1Block: 50,
});

assert.strictEqual(metrics[0].value, '60.0s');
assert.strictEqual(metrics[0].group, 'Network Performance');
assert.strictEqual(metrics[1].value, 'N/A');
assert.strictEqual(metrics[1].group, 'Network Performance');
assert.strictEqual(metrics[2].value, '1.20s');
assert.strictEqual(metrics[2].group, 'Network Performance');
assert.strictEqual(metrics[3].value, 'N/A');
assert.strictEqual(metrics[3].group, 'Network Performance');
assert.strictEqual(metrics[4].value, '2');
assert.strictEqual(metrics[4].group, 'Operators');
assert.strictEqual(metrics[5].value, '0xabc');
assert.strictEqual(metrics[5].group, 'Operators');
assert.strictEqual(metrics[6].value, 'N/A');
assert.strictEqual(metrics[6].group, 'Operators');
assert.strictEqual(metrics[7].value, '1');
assert.strictEqual(metrics[7].group, 'Network Health & Security');
assert.strictEqual(metrics[8].value, 'N/A');
assert.strictEqual(metrics[8].group, 'Network Health & Security');
assert.strictEqual(metrics[9].value, '0');
assert.strictEqual(metrics[9].group, 'Network Health & Security');
assert.strictEqual(metrics[10].value, '100');
assert.strictEqual(metrics[11].value, '50');

const results = [
  { badRequest: false, data: null },
  { badRequest: true, data: null },
];
assert.strictEqual(hasBadRequest(results), true);
assert.strictEqual(hasBadRequest([{ badRequest: false, data: null }]), false);

const metricsAllNull = createMetrics({
  l2Cadence: null,
  batchCadence: null,
  avgProve: null,
  avgVerify: null,
  activeGateways: null,
  l2Reorgs: null,
  slashings: null,
  forcedInclusions: null,
  l2Block: null,
  l1Block: null,
  currentOperator: null,
  nextOperator: null,
});
for (const metric of metricsAllNull) {
  assert.strictEqual(metric.value, 'N/A');
}
assert.strictEqual(metricsAllNull[0].group, 'Network Performance');
assert.strictEqual(metricsAllNull[1].group, 'Network Performance');
assert.strictEqual(metricsAllNull[2].group, 'Network Performance');
assert.strictEqual(metricsAllNull[3].group, 'Network Performance');
assert.strictEqual(metricsAllNull[4].group, 'Operators');
assert.strictEqual(metricsAllNull[5].group, 'Operators');
assert.strictEqual(metricsAllNull[6].group, 'Operators');
assert.strictEqual(metricsAllNull[7].group, 'Network Health & Security');
assert.strictEqual(metricsAllNull[8].group, 'Network Health & Security');
assert.strictEqual(metricsAllNull[9].group, 'Network Health & Security');

assert.strictEqual(
  hasBadRequest([
    { badRequest: false, data: null },
    { badRequest: false, data: null },
  ]),
  false,
);

console.log('Helper tests passed.');
