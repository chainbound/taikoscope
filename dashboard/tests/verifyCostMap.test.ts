import { describe, it, expect } from 'vitest';
import { TABLE_CONFIGS } from '../config/tableConfig';

describe('verify-cost mapData', () => {
  const mapData = TABLE_CONFIGS['verify-cost'].mapData!;

  it('formats batch number and cost correctly', () => {
    const rows = mapData([{ batch: 2000, cost: 21e18 }]);
    expect(rows[0].batch).toBe('2,000');
    expect(rows[0].cost).toBe('21.0 ETH');
  });
});
