import { describe, it, expect } from 'vitest';
import { TABLE_CONFIGS } from '../config/tableConfig';

describe('l2-gas-used mapData', () => {
  const mapData = TABLE_CONFIGS['l2-gas-used'].mapData!;

  it('formats block number and gas used with locale commas', () => {
    const rows = mapData([{ value: 1660970, timestamp: 268542 }]);
    expect(rows[0].value).toBe('1,660,970');
    expect(rows[0].timestamp).toBe('268,542');
  });
});
