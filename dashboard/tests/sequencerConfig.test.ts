import { describe, it, expect } from 'vitest';
import { getSequencerName } from '../sequencerConfig';

const addressA = '0x00a00800c28f2616360dcfadee02d761d14ad94e';
const unknown = '0xdeadbeef';

describe('getSequencerName', () => {
  it('returns known sequencer name', () => {
    expect(getSequencerName(addressA)).toBe('Chainbound A');
  });

  it('returns "Unknown" for unmapped address', () => {
    expect(getSequencerName(unknown)).toBe('Unknown');
  });
});
