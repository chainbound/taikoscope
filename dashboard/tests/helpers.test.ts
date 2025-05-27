import assert from "assert";
import { createMetrics, hasBadRequest } from "../helpers.js";

const metrics = createMetrics({
  l2Cadence: 60000,
  batchCadence: null,
  avgProve: 1200,
  avgVerify: 0,
  activeGateways: 2,
  l2Reorgs: 1,
  slashings: null,
  forcedInclusions: 0,
  l2Block: 100,
  l1Block: 50,
});

assert.strictEqual(metrics[0].value, "60.0s");
assert.strictEqual(metrics[1].value, "N/A");
assert.strictEqual(metrics[2].value, "1.20s");
assert.strictEqual(metrics[3].value, "N/A");
assert.strictEqual(metrics[4].value, "2");
assert.strictEqual(metrics[5].value, "1");
assert.strictEqual(metrics[6].value, "N/A");
assert.strictEqual(metrics[7].value, "0");
assert.strictEqual(metrics[8].value, "100");
assert.strictEqual(metrics[9].value, "50");

const results = [
  { badRequest: false, data: null },
  { badRequest: true, data: null },
];
assert.strictEqual(hasBadRequest(results), true);
assert.strictEqual(hasBadRequest([{ badRequest: false, data: null }]), false);

console.log("Helper tests passed.");
