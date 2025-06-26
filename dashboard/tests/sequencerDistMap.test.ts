import { describe, it, expect } from 'vitest';
import React from 'react';
import { TABLE_CONFIGS } from '../config/tableConfig';

describe('sequencer-dist mapData', () => {
  const mapData = TABLE_CONFIGS['sequencer-dist'].mapData!;

  it('formats zero TPS correctly', () => {
    const rows = mapData([{ name: 'foo', value: 1, tps: 0 }]);
    expect(rows[0].tps).toBe('0.00');
  });

  it('wraps the sequencer name in a link', () => {
    const rows = mapData([{ name: 'foo', value: 1, tps: 1 }]);
    const el = rows[0].name as React.ReactElement;
    expect(el.type).toBe('a');
  });

  it('handles missing TPS as N/A', () => {
    const rows = mapData([{ name: 'foo', value: 1, tps: null }]);
    expect(rows[0].tps).toBe('N/A');
  });
});
