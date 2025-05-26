import {
  TimeRange,
  TimeSeriesData,
  PieChartDataItem,
  MetricData,
} from "../types";

// Example color palette if needed by future charts
// const TAIKO_PINK = '#BF007C';
// const COLORS = [TAIKO_PINK, '#E573B5', '#5DA5DA', '#FAA43A', '#60BD68', '#F17CB0', '#B2912F', '#B276B2', '#DECF3F', '#F15854'];

const getRandom = (min: number, max: number, float = false): number => {
  const val = Math.random() * (max - min) + min;
  return float ? val : Math.floor(val);
};

export const generateMockMetrics = (timeRange: TimeRange): MetricData[] => {
  const factor = timeRange === "1h" ? 1 : timeRange === "24h" ? 24 : 24 * 7;
  return [
    {
      title: "L2 Block Cadence",
      value: `${getRandom(1.91, 2.11, true).toFixed(2)}s`,
    },
    { title: "Active Gateways", value: "3" }, // Hardcoded as per request
    { title: "L2 Reorgs", value: getRandom(0, 1 * factor).toString() },
    {
      title: "Batch Posting Cadence",
      value: `${getRandom(40, 60, true).toFixed(2)}s`,
    },
    { title: "Slashing Events", value: getRandom(0, 1 * factor).toString() },
    { title: "Forced Inclusions", value: getRandom(0, 2 * factor).toString() },
    {
      title: "Avg. Prove Time",
      value: `${getRandom(600, 700, true).toFixed(2)}s`,
    },
    {
      title: "Avg. Verify Time",
      value: `${getRandom(10, 15, true).toFixed(2)}s`,
    },
    {
      title: "L2 Head Block",
      value: (266226 + getRandom(0, 100 * factor)).toLocaleString(),
    },
    {
      title: "L1 Head Block",
      value: (22487468 + getRandom(0, 50 * factor)).toLocaleString(),
    },
  ];
};

export const generateMockBlockTimeData = (
  timeRange: TimeRange,
  type: "L1" | "L2",
): TimeSeriesData[] => {
  const now = Date.now();
  const points = timeRange === "1h" ? 12 : timeRange === "24h" ? 24 : 7; // hourly for 24h, daily for 7d
  const interval =
    timeRange === "1h"
      ? 5 * 60 * 1000
      : timeRange === "24h"
        ? 60 * 60 * 1000
        : 24 * 60 * 60 * 1000;

  let startBlock = type === "L2" ? 165000 : 3877350;
  const increment = type === "L2" ? 100 : 50;

  return Array.from({ length: points })
    .map((_, i) => {
      startBlock += getRandom(increment * 0.8, increment * 1.2);
      return {
        timestamp: now - (points - 1 - i) * interval,
        value: startBlock,
      };
    })
    .sort((a, b) => a.timestamp - b.timestamp);
};

export const generateMockBatchProcessData = (
  timeRange: TimeRange,
  type: "prove" | "verify",
): TimeSeriesData[] => {
  const points = timeRange === "1h" ? 10 : timeRange === "24h" ? 20 : 30;
  let startBatchId = 5800;

  const rangeMillis =
    timeRange === "1h"
      ? 60 * 60 * 1000
      : timeRange === "24h"
        ? 24 * 60 * 60 * 1000
        : 7 * 24 * 60 * 60 * 1000;

  return Array.from({ length: points }).map((_, i) => {
    startBatchId += getRandom(1, 3);
    return {
      name: (startBatchId + i).toString(),
      value: type === "prove" ? getRandom(500, 2500) : getRandom(30, 150),
      timestamp: Date.now() - (points - 1 - i) * (rangeMillis / points), // spread over the range
    };
  });
};

export const generateMockSequencerData = (): PieChartDataItem[] => {
  // Specific sequencers as requested
  const sequencers = [
    { name: "Titan", value: getRandom(800, 1500) },
    { name: "Nethermind", value: getRandom(400, 900) },
    { name: "Chainbound", value: getRandom(200, 600) },
  ];

  return sequencers
    .map((s) => ({
      name: s.name,
      value: s.value,
      // fill property is now optional and will be assigned by the PieChart component
    }))
    .sort((a, b) => b.value - a.value);
};
