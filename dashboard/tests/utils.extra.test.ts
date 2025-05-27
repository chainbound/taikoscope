import assert from 'assert';
import { formatDecimal, formatSeconds } from '../utils.js';

assert.strictEqual(formatDecimal(-5.678), '-5.68');
assert.strictEqual(formatSeconds(0), '0.00s');
assert.strictEqual(formatSeconds(119), '119.0s');
assert.strictEqual(formatSeconds(120), '2m');

console.log('Extra util tests passed.');
