import { describe, it, expect } from 'vitest';
import { TABLE_CONFIGS } from '../config/tableConfig';

describe('l2-gas-used mapData', () => {
  const mapData = TABLE_CONFIGS['l2-gas-used'].mapData!;

  it('formats block number as clickable link and gas used with locale commas', () => {
    const rows = mapData([{ value: 1660970, timestamp: 268542 }]);

    // Check that value is a React element (blockLink)
    expect(typeof rows[0].value).toBe('object');
    expect(rows[0].value).toHaveProperty('props');

    // Check that timestamp is formatted with locale commas
    expect(rows[0].timestamp).toBe('268,542');
  });
});
