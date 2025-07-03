import { describe, it, expect } from 'vitest';
import { TABLE_CONFIGS } from '../config/tableConfig';

describe('prove-cost mapData', () => {
  const mapData = TABLE_CONFIGS['prove-cost'].mapData!;

  it('formats batch number and cost correctly', () => {
    const rows = mapData([{ batch: 1000, cost: 42e9 }]);
    expect(rows[0].batch).toBe('1,000');
    expect(rows[0].cost).toBe('42 ETH');
  });
});
