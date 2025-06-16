import { describe, it, expect } from 'vitest';
import { TABLE_CONFIGS } from '../config/tableConfig';

describe('sequencer-dist mapData', () => {
  const mapData = TABLE_CONFIGS['sequencer-dist'].mapData!;

  it('formats zero TPS correctly', () => {
    const rows = mapData([{ name: 'foo', value: 1, tps: 0 }]);
    expect(rows[0].tps).toBe('0.00');
  });

  it('handles missing TPS as N/A', () => {
    const rows = mapData([{ name: 'foo', value: 1, tps: null }]);
    expect(rows[0].tps).toBe('N/A');
  });
});
