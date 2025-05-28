import assert from 'assert';
import {
  formatDecimal,
  formatSeconds,
  formatInterval,
  formatBatchDuration,
  computeBatchDurationFlags,
  shouldShowMinutes,
  findMetricValue,
  formatSequencerTooltip,
  bytesToHex,
} from '../utils.js';

assert.strictEqual(formatDecimal(1), '1.00');
assert.strictEqual(formatDecimal(12.345), '12.3');

assert.strictEqual(formatSeconds(30), '30.0s');
assert.strictEqual(formatSeconds(150), '2.5m');
assert.strictEqual(formatSeconds(7200), '2h');

assert.strictEqual(formatInterval(30000, false), '30 seconds');
assert.strictEqual(formatInterval(180000, true), '3.00 minutes');

assert.strictEqual(formatBatchDuration(45, false, false), '45 seconds');
assert.strictEqual(formatBatchDuration(150, false, true), '2.50 minutes');
assert.strictEqual(formatBatchDuration(7200, true, false), '2.00 hours');

const flags = computeBatchDurationFlags([{ value: 30 }, { value: 7200 }]);
assert.strictEqual(flags.showHours, true);
assert.strictEqual(flags.showMinutes, false);

const flagsMinutes = computeBatchDurationFlags([
  { value: 150 },
  { value: 100 },
]);
assert.strictEqual(flagsMinutes.showHours, false);
assert.strictEqual(flagsMinutes.showMinutes, true);

const flagsNone = computeBatchDurationFlags([{ value: 60 }, { value: 80 }]);
assert.strictEqual(flagsNone.showHours, false);
assert.strictEqual(flagsNone.showMinutes, false);

assert.strictEqual(
  shouldShowMinutes([{ timestamp: 1000 }, { timestamp: 200000 }]),
  true,
);

assert.strictEqual(
  shouldShowMinutes([{ timestamp: 1000 }, { timestamp: 110000 }]),
  false,
);

const metrics = [
  { title: 'Test', value: '42' },
  { title: 'Another', value: '0' },
];
assert.strictEqual(findMetricValue(metrics, 'test'), '42');
assert.strictEqual(findMetricValue(metrics, 'TEST'), '42');
assert.strictEqual(findMetricValue(metrics, 'missing'), 'N/A');

const tooltip = formatSequencerTooltip([{ value: 1 }, { value: 3 }], 1);
assert.strictEqual(tooltip, '1 blocks (25.00%)');

const zeroTooltip = formatSequencerTooltip([{ value: 0 }], 0);
assert.strictEqual(zeroTooltip, '0 blocks (0%)');

assert.strictEqual(bytesToHex([0, 1, 255]), '0x0001ff');

console.log('All tests passed.');
