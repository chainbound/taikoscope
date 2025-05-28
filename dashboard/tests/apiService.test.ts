import assert from 'assert';
import fs from 'fs/promises';

let fetchAvgProveTime: typeof import('../services/apiService.js').fetchAvgProveTime;
let fetchActiveGateways: typeof import('../services/apiService.js').fetchActiveGateways;
let fetchL2BlockTimes: typeof import('../services/apiService.js').fetchL2BlockTimes;

const originalFetch = globalThis.fetch;

async function loadService() {
  const path = new URL('../services/apiService.js', import.meta.url);
  let code = await fs.readFile(path, 'utf8');
  if (!code.startsWith('import.meta.env ??=')) {
    code = 'import.meta.env ??= {};' + '\n' + code;
    await fs.writeFile(path, code);
  }
  return import(path.href + '?patched=' + Date.now());
}

// helper to create mock fetch response
function mockFetch(data: unknown, status = 200, ok = true) {
  return async () =>
    ({
      ok,
      status,
      json: async () => data,
    }) as Response;
}

(async () => {
  const service = await loadService();
  fetchAvgProveTime = service.fetchAvgProveTime;
  fetchActiveGateways = service.fetchActiveGateways;
  fetchL2BlockTimes = service.fetchL2BlockTimes;
  // success case for fetchAvgProveTime
  globalThis.fetch = mockFetch({ avg_prove_time_ms: 42 });
  const prove = await fetchAvgProveTime('1h');
  assert.strictEqual(prove.badRequest, false);
  assert.strictEqual(prove.data, 42);

  // bad request handling for fetchAvgProveTime
  globalThis.fetch = mockFetch({}, 400, false);
  const badProve = await fetchAvgProveTime('1h');
  assert.strictEqual(badProve.badRequest, true);
  assert.strictEqual(badProve.data, null);

  // transformation for fetchActiveGateways
  globalThis.fetch = mockFetch({ gateways: ['a', 'b'] });
  const gateways = await fetchActiveGateways('1h');
  assert.strictEqual(gateways.data, 2);

  // l2 block times transformation
  globalThis.fetch = mockFetch({
    blocks: [
      { l2_block_number: 1, ms_since_prev_block: 10 },
      { l2_block_number: 2, ms_since_prev_block: 20 },
    ],
  });
  const blockTimes = await fetchL2BlockTimes('1h');
  assert.deepStrictEqual(blockTimes.data, [{ value: 2, timestamp: 20 }]);

  // clean up
  globalThis.fetch = originalFetch;
  console.log('apiService tests passed.');
})();
