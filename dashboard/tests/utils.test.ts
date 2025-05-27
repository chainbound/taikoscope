import assert from "assert";
import {
  formatDecimal,
  formatSeconds,
  formatInterval,
  formatBatchDuration,
} from "../utils.js";

assert.strictEqual(formatDecimal(1), "1.00");
assert.strictEqual(formatDecimal(12.345), "12.3");

assert.strictEqual(formatSeconds(30), "30.0s");
assert.strictEqual(formatSeconds(150), "2.5m");
assert.strictEqual(formatSeconds(7200), "2h");

assert.strictEqual(formatInterval(30000, false), "30 seconds");
assert.strictEqual(formatInterval(180000, true), "3.00 minutes");

assert.strictEqual(formatBatchDuration(45, false, false), "45 seconds");
assert.strictEqual(formatBatchDuration(150, false, true), "2.50 minutes");
assert.strictEqual(formatBatchDuration(7200, true, false), "2.00 hours");

console.log("All tests passed.");
