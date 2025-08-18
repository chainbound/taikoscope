import { describe, it, expect } from 'vitest';
import {
  convertFeesToUsd,
  calculateSequencerData,
  processSequencerData,
  generateFallbackSankeyData,
  generateMultiSequencerSankeyData,
  validateChartData,
  safeValue,
  type SequencerFeeData,
  type ProcessedSequencerData,
  type SankeyChartData,
} from '../utils/feeFlowCalculations';

describe('feeFlowCalculations', () => {
  describe('safeValue', () => {
    it('should return the value if it is finite', () => {
      expect(safeValue(42)).toBe(42);
      expect(safeValue(0)).toBe(0);
      expect(safeValue(-42)).toBe(-42);
      expect(safeValue(3.14159)).toBe(3.14159);
    });

    it('should return 0 for non-finite values', () => {
      expect(safeValue(NaN)).toBe(0);
      expect(safeValue(Infinity)).toBe(0);
      expect(safeValue(-Infinity)).toBe(0);
    });
  });

  describe('convertFeesToUsd', () => {
    const ethPrice = 2000;

    it('should convert fees from Gwei to USD', () => {
      const result = convertFeesToUsd({
        priorityFee: 1e9, // 1 ETH worth of Gwei
        baseFee: 2e9,    // 2 ETH worth of Gwei
        l1DataCost: 5e8, // 0.5 ETH worth of Gwei
        proveCost: 3e8,  // 0.3 ETH worth of Gwei
        ethPrice,
      });

      expect(result.priorityFeeUsd).toBe(2000); // 1 ETH * $2000
      expect(result.baseFeeUsd).toBe(4000);     // 2 ETH * $2000
      expect(result.l1DataCostTotalUsd).toBe(1000); // 0.5 ETH * $2000
      expect(result.l1ProveCost).toBe(600);     // 0.3 ETH * $2000
      expect(result.baseFeeDaoUsd).toBe(1000);  // 25% of base fee
    });

    it('should handle null/undefined fee values', () => {
      const result = convertFeesToUsd({
        priorityFee: null,
        baseFee: undefined,
        l1DataCost: null,
        proveCost: undefined,
        ethPrice,
      });

      expect(result.priorityFeeUsd).toBe(0);
      expect(result.baseFeeUsd).toBe(0);
      expect(result.l1DataCostTotalUsd).toBe(0);
      expect(result.l1ProveCost).toBe(0);
      expect(result.baseFeeDaoUsd).toBe(0);
    });

    it('should handle zero ETH price', () => {
      const result = convertFeesToUsd({
        priorityFee: 1e9,
        baseFee: 2e9,
        l1DataCost: 5e8,
        proveCost: 3e8,
        ethPrice: 0,
      });

      expect(result.priorityFeeUsd).toBe(0);
      expect(result.baseFeeUsd).toBe(0);
      expect(result.l1DataCostTotalUsd).toBe(0);
      expect(result.l1ProveCost).toBe(0);
      expect(result.baseFeeDaoUsd).toBe(0);
    });

    it('should handle NaN values gracefully', () => {
      const result = convertFeesToUsd({
        priorityFee: NaN,
        baseFee: Infinity,
        l1DataCost: -Infinity,
        proveCost: 1e9,
        ethPrice: NaN,
      });

      // All results should be 0 due to safeValue protection
      expect(result.priorityFeeUsd).toBe(0);
      expect(result.baseFeeUsd).toBe(0);
      expect(result.l1DataCostTotalUsd).toBe(0);
      expect(result.l1ProveCost).toBe(0);
      expect(result.baseFeeDaoUsd).toBe(0);
    });
  });

  describe('calculateSequencerData', () => {
    const mockSequencerFee: SequencerFeeData = {
      address: '0x1234567890123456789012345678901234567890',
      priority_fee: 1e9,  // 1 ETH worth of Gwei
      base_fee: 2e9,      // 2 ETH worth of Gwei
      l1_data_cost: 5e8,  // 0.5 ETH worth of Gwei
      prove_cost: 3e8,    // 0.3 ETH worth of Gwei
    };

    const ethPrice = 2000;
    const hardwareCostPerSeq = 100;

    it('should calculate sequencer data correctly', () => {
      const result = calculateSequencerData(mockSequencerFee, ethPrice, hardwareCostPerSeq);

      expect(result.address).toBe(mockSequencerFee.address);
      expect(result.shortAddress).toBe('0x12345'); // First 7 chars
      expect(result.priorityUsd).toBe(2000); // 1 ETH * $2000
      expect(result.baseUsd).toBe(3000);     // 1.5 ETH * $2000 (75% of 2 ETH)
      expect(result.revenue).toBe(5000);     // priority + base
      expect(result.l1CostUsd).toBe(1000);   // 0.5 ETH * $2000
      expect(result.actualHardwareCost).toBe(hardwareCostPerSeq);
    });

    it('should handle subsidies when costs exceed revenue', () => {
      const highCostSequencer: SequencerFeeData = {
        ...mockSequencerFee,
        priority_fee: 1e8,  // 0.1 ETH (low revenue) worth of Gwei
        base_fee: 1e8,      // 0.1 ETH worth of Gwei
        l1_data_cost: 5e9,  // 5 ETH (high L1 cost) worth of Gwei
        prove_cost: 2e9,    // 2 ETH (high prove cost) worth of Gwei
      };

      const result = calculateSequencerData(highCostSequencer, ethPrice, 1000); // High hardware cost

      // Revenue should be less than total costs in this scenario
      expect(result.subsidyUsd).toBeGreaterThan(0);
      expect(result.profit).toBeLessThan(0); // Should have negative profit
    });

    it('should handle null fee values', () => {
      const nullFeeSequencer: SequencerFeeData = {
        address: '0x1234567890123456789012345678901234567890',
        priority_fee: null,
        base_fee: null,
        l1_data_cost: null,
        prove_cost: null,
      };

      const result = calculateSequencerData(nullFeeSequencer, ethPrice, hardwareCostPerSeq);

      expect(result.priorityUsd).toBe(0);
      expect(result.baseUsd).toBe(0);
      expect(result.revenue).toBe(0);
      expect(result.l1CostUsd).toBe(0);
      // With no revenue but hardware cost of 100, there will be a subsidy needed
      expect(result.subsidyUsd).toBe(hardwareCostPerSeq); // Deficit from hardware cost
    });

    it('should calculate subsidy as total costs minus revenue when revenue is insufficient', () => {
      // Create scenario where sequencer can't cover all L1 costs and has deficit
      const lowRevenueSequencer: SequencerFeeData = {
        address: '0x1234567890123456789012345678901234567890',
        priority_fee: 1e8,   // 0.1 ETH worth of Gwei
        base_fee: 1e8,       // 0.1 ETH worth of Gwei
        l1_data_cost: 1e9,   // 1 ETH L1 cost worth of Gwei
        prove_cost: 5e8,     // 0.5 ETH prove cost worth of Gwei
      };

      const result = calculateSequencerData(lowRevenueSequencer, ethPrice, 500); // $500 hardware cost

      // Revenue: 0.1 + 0.075 = 0.175 ETH = $350
      // Costs: Hardware $500 + L1 $2000 + Prove $1000 = $3500 total
      // Subsidy should be max(0, total costs - revenue) = $3150
      const expectedSubsidy = 3500 - 350;
      expect(result.subsidyUsd).toBe(expectedSubsidy);
    });
  });

  describe('processSequencerData', () => {
    const mockSequencerFees: SequencerFeeData[] = [
      {
        address: '0x1111111111111111111111111111111111111111',
        priority_fee: 2e9,
        base_fee: 3e9,
        l1_data_cost: 1e9,
        prove_cost: 5e8,
      },
      {
        address: '0x2222222222222222222222222222222222222222',
        priority_fee: 1e9,
        base_fee: 2e9,
        l1_data_cost: 2e9,
        prove_cost: 1e9,
      },
    ];

    it('should process all sequencer data and sort by profitability', () => {
      const result = processSequencerData(mockSequencerFees, 2000, 100);

      expect(result).toHaveLength(2);
      expect(result[0].address).toBe(mockSequencerFees[1].address); // Lower profit first
      expect(result[1].address).toBe(mockSequencerFees[0].address); // Higher profit second
      expect(result[0].profit).toBeLessThanOrEqual(result[1].profit);
    });

    it('should handle empty sequencer array', () => {
      const result = processSequencerData([], 2000, 100);
      expect(result).toHaveLength(0);
    });

    it('should aggregate multiple addresses for the same operator', () => {
      // Mock addresses that map to the same operator name (Gattaca in sequencerConfig.ts)
      const multiAddressFees: SequencerFeeData[] = [
        {
          address: '0xe2dA8aC2E550cd141198a117520D4EDc8692AB74', // Gattaca address 1
          priority_fee: 1e9,
          base_fee: 2e9,
          l1_data_cost: 5e8,
          prove_cost: 0,
        },
        {
          address: '0x2C89DC1b6ECA603AdaCe60A76d3074F3835f6cBE', // Gattaca address 2
          priority_fee: 1e9,
          base_fee: 2e9,
          l1_data_cost: 0,
          prove_cost: 0,
        },
      ];

      const result = processSequencerData(multiAddressFees, 2000, 100);

      // Should only have one result for Gattaca (aggregated)
      expect(result).toHaveLength(1);
      expect(result[0].shortAddress).toBe('Gattaca');

      // Fees should be aggregated: 1e9 + 1e9 = 2e9 priority_fee, 2e9 + 2e9 = 4e9 base_fee
      const expectedRevenueGwei = 2e9 + 4e9 * 0.75; // priority + base * 0.75
      expect(result[0].revenueGwei).toBe(expectedRevenueGwei);

      // Hardware cost should be doubled (100 * 2 addresses)
      expect(result[0].actualHardwareCost).toBe(200);
    });
  });

  describe('generateFallbackSankeyData', () => {
    const fallbackParams = {
      priorityFeeUsd: 1000,
      baseFeeUsd: 2000,
      baseFeeDaoUsd: 500,
      l1DataCostTotalUsd: 800,
      l1ProveCost: 400,
      totalHardwareCost: 300,
      priorityFee: 5e8,
      baseFee: 1e9,
      ethPrice: 2000,
    };

    it('should generate fallback Sankey data structure', () => {
      const result = generateFallbackSankeyData(fallbackParams);

      expect(result.nodes).toBeDefined();
      expect(result.links).toBeDefined();
      expect(result.nodes.length).toBeGreaterThan(0);
      expect(result.links.length).toBeGreaterThan(0);

      // Check for required nodes
      const nodeNames = result.nodes.map(n => n.name);
      expect(nodeNames).toContain('Priority Fee');
      expect(nodeNames).toContain('Base Fee');
      expect(nodeNames).toContain('Sequencers');
      expect(nodeNames).toContain('Taiko DAO');
      expect(nodeNames).toContain('Hardware Cost');
      expect(nodeNames).toContain('Proving Cost');
      expect(nodeNames).toContain('Proposing Cost');
      expect(nodeNames).toContain('Profit');
    });

    it('should include subsidy node when L1 costs exceed available revenue', () => {
      const highCostParams = {
        ...fallbackParams,
        l1DataCostTotalUsd: 3000, // Higher than available revenue after hardware costs
      };

      const result = generateFallbackSankeyData(highCostParams);
      const nodeNames = result.nodes.map(n => n.name);
      expect(nodeNames).toContain('Subsidy');

      const subsidyNode = result.nodes.find(n => n.name === 'Subsidy');
      expect(subsidyNode?.value).toBeGreaterThan(0);
    });

    it('should filter out zero-value links', () => {
      const result = generateFallbackSankeyData(fallbackParams);

      result.links.forEach(link => {
        expect(link.value).toBeGreaterThan(0);
      });
    });
  });

  describe('generateMultiSequencerSankeyData', () => {
    const mockProcessedData: ProcessedSequencerData[] = [
      {
        address: '0x1111111111111111111111111111111111111111',
        shortAddress: '0x11111',
        priorityUsd: 1000,
        baseUsd: 1500,
        revenue: 2500,
        revenueGwei: 1.25e9,
        profit: 500,
        profitGwei: 2.5e8,
        actualHardwareCost: 300,
        actualL1Cost: 800,
        actualProveCost: 400,
        l1CostUsd: 1000,
        subsidyUsd: 200,
        subsidyGwei: 1e8,
        actualHardwareCostGwei: 1.5e8,
        actualL1CostGwei: 4e8,
        actualProveCostGwei: 2e8,
      },
      {
        address: '0x2222222222222222222222222222222222222222',
        shortAddress: '0x22222',
        priorityUsd: 800,
        baseUsd: 1200,
        revenue: 2000,
        revenueGwei: 1e9,
        profit: 200,
        profitGwei: 1e8,
        actualHardwareCost: 300,
        actualL1Cost: 600,
        actualProveCost: 300,
        l1CostUsd: 800,
        subsidyUsd: 200,
        subsidyGwei: 1e8,
        actualHardwareCostGwei: 1.5e8,
        actualL1CostGwei: 3e8,
        actualProveCostGwei: 1.5e8,
      },
    ];

    it('should generate multi-sequencer Sankey data structure', () => {
      const result = generateMultiSequencerSankeyData(
        mockProcessedData,
        1800, // priorityFeeUsd
        2700, // baseFeeUsd
        675,  // baseFeeDaoUsd
        2.25e9, // priorityFee
        3.375e9, // baseFee
      );

      expect(result.nodes).toBeDefined();
      expect(result.links).toBeDefined();
      expect(result.nodes.length).toBeGreaterThan(mockProcessedData.length + 3); // At least sequencers + fee sources + costs + profit

      // Check for sequencer nodes
      const sequencerNodes = result.nodes.filter(n =>
        mockProcessedData.some(s => s.shortAddress === n.name)
      );
      expect(sequencerNodes).toHaveLength(mockProcessedData.length);
    });

    it('should create proper links between all components', () => {
      const result = generateMultiSequencerSankeyData(
        mockProcessedData,
        1800,
        2700,
        675,
        2.25e9,
        3.375e9,
      );

      // Should have links from fee sources to sequencers
      const feeToSequencerLinks = result.links.filter(l =>
        l.source <= 2 && l.target >= 3 && l.target < 3 + mockProcessedData.length
      );
      expect(feeToSequencerLinks.length).toBeGreaterThan(0);

      // Should have links from sequencers to costs/profit
      const sequencerToOutputLinks = result.links.filter(l =>
        l.source >= 3 && l.source < 3 + mockProcessedData.length
      );
      expect(sequencerToOutputLinks.length).toBeGreaterThan(0);
    });

    it('should aggregate totals correctly', () => {
      const result = generateMultiSequencerSankeyData(
        mockProcessedData,
        1800,
        2700,
        675,
        2.25e9,
        3.375e9,
      );

      const profitNode = result.nodes.find(n => n.profitNode);
      // Profit node now aggregates only positive profits (loss-makers contribute 0; covered by subsidy)
      const expectedTotalProfit = mockProcessedData
        .reduce((acc, s) => acc + Math.max(0, s.profit), 0);

      // The function may add epsilon values to ensure visual connectivity
      // Allow for reasonable epsilon additions (up to 5% of expected profit or 50, whichever is larger)
      const tolerance = Math.max(expectedTotalProfit * 0.05, 50);
      expect(profitNode?.value).toBeGreaterThanOrEqual(expectedTotalProfit);
      expect(profitNode?.value).toBeLessThanOrEqual(expectedTotalProfit + tolerance);
    });
  });

  describe('validateChartData', () => {
    it('should return empty data for invalid input', () => {
      const invalidData: SankeyChartData = {
        nodes: [],
        links: [],
      };

      const result = validateChartData(invalidData);
      expect(result.nodes).toHaveLength(0);
      expect(result.links).toHaveLength(0);
    });

    it('should filter out invalid links', () => {
      const testData: SankeyChartData = {
        nodes: [
          { name: 'Node1', value: 100, depth: 0 },
          { name: 'Node2', value: 200, depth: 1 },
          { name: 'Node3', value: 150, depth: 2 },
        ],
        links: [
          { source: 0, target: 1, value: 100 },    // Valid
          { source: 1, target: 2, value: 150 },    // Valid
          { source: 0, target: 5, value: 50 },     // Invalid target index
          { source: -1, target: 1, value: 30 },    // Invalid source index
          { source: 1, target: 2, value: 0 },      // Invalid zero value
          { source: 0, target: 2, value: NaN },    // Invalid NaN value
        ],
      };

      const result = validateChartData(testData);

      expect(result.links).toHaveLength(2); // Only the first two valid links
      expect(result.nodes).toHaveLength(3); // All nodes should remain
    });

    it('should remove unused nodes after link filtering', () => {
      const testData: SankeyChartData = {
        nodes: [
          { name: 'Node1', value: 100, depth: 0 },
          { name: 'Node2', value: 200, depth: 1 },
          { name: 'Node3', value: 150, depth: 2 },
          { name: 'UnusedNode', value: 50, depth: 3 },
        ],
        links: [
          { source: 0, target: 1, value: 100 }, // Only connects first two nodes
        ],
      };

      const result = validateChartData(testData);

      expect(result.nodes).toHaveLength(2); // Unused nodes should be removed
      expect(result.links).toHaveLength(1);
      expect(result.links[0].source).toBe(0);
      expect(result.links[0].target).toBe(1);
    });

    it('should remap node indices after filtering', () => {
      const testData: SankeyChartData = {
        nodes: [
          { name: 'Node1', value: 100, depth: 0 },
          { name: 'UnusedNode', value: 50, depth: 1 },
          { name: 'Node3', value: 150, depth: 2 },
        ],
        links: [
          { source: 0, target: 2, value: 100 }, // Skip middle node
        ],
      };

      const result = validateChartData(testData);

      expect(result.nodes).toHaveLength(2);
      expect(result.links).toHaveLength(1);
      expect(result.links[0].source).toBe(0);
      expect(result.links[0].target).toBe(1); // Should be remapped from 2 to 1
    });

    it('should apply safeValue to node and link values', () => {
      const testData: SankeyChartData = {
        nodes: [
          { name: 'Node1', value: NaN, depth: 0 },
          { name: 'Node2', value: Infinity, depth: 1, gwei: -Infinity },
        ],
        links: [
          { source: 0, target: 1, value: -Infinity },
        ],
      };

      const result = validateChartData(testData);

      // After applying safeValue, infinite values should become 0 and be filtered out
      expect(result.nodes).toHaveLength(0);
      expect(result.links).toHaveLength(0);
    });
  });
});