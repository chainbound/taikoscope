import { describe, it, expect } from 'vitest';
import { TABLE_CONFIGS } from '../config/tableConfig';

describe('l1-data-cost mapData', () => {
  const mapData = TABLE_CONFIGS['l1-data-cost'].mapData!;

  it('formats block number as link and cost as ETH', () => {
    const rows = mapData([{ block_number: 100, cost: 42e9 }]);

    // value should be React element from blockLink
    expect(typeof rows[0].block).toBe('object');
    expect(rows[0].block).toHaveProperty('props');
    expect(rows[0].cost).toBe('42 ETH');
  });
});
