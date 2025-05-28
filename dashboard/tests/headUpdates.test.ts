import assert from 'assert';
import { setupBlockNumberUpdates } from '../headUpdates.js';

class MockEventSource {
  url: string;
  onmessage: ((e: { data: string }) => void) | null = null;
  onerror: (() => void) | null = null;
  closed = false;

  constructor(url: string) {
    this.url = url;
  }

  close() {
    this.closed = true;
  }
}

type IntervalFn = (fn: () => void, delay: number) => number;

type ClearFn = (id: number) => void;

const events: MockEventSource[] = [];
const factory = (url: string) => {
  const es = new MockEventSource(url);
  events.push(es);
  return es as unknown as EventSource;
};

let intervalCalled: [() => void, number] | null = null;
const setIntervalFn: IntervalFn = (fn, delay) => {
  intervalCalled = [fn, delay];
  return 1;
};

let cleared: number | null = null;
const clearFn: ClearFn = (id) => {
  cleared = id;
};

let updates = 0;
const updateHeads = () => {
  updates += 1;
};

let errorMsg = '';
const cleanup = setupBlockNumberUpdates({
  onL1Message: () => {},
  onL2Message: () => {},
  updateHeads,
  setErrorMessage: (m) => {
    errorMsg = m;
  },
  eventSourceFactory: factory,
  setIntervalFn,
  clearIntervalFn: clearFn,
});

assert.strictEqual(events.length, 2);

// trigger error
if (events[0].onerror) {
  events[0].onerror();
}

assert.strictEqual(errorMsg.includes('falling back to polling'), true);
assert.strictEqual(updates, 1);
assert.deepStrictEqual(intervalCalled?.[1], 10000);

cleanup();
assert.strictEqual(events[0].closed, true);
assert.strictEqual(cleared, 1);

console.log('headUpdates tests passed.');
