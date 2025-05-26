import React, { useState, useEffect, useCallback } from "react";
import { DashboardHeader } from "./components/DashboardHeader";
import { MetricCard } from "./components/MetricCard";
import { ChartCard } from "./components/ChartCard";
import { SequencerPieChart } from "./components/SequencerPieChart";
import { BlockTimeChart } from "./components/BlockTimeChart";
import { BatchProcessChart } from "./components/BatchProcessChart";
import {
  TimeRange,
  TimeSeriesData,
  PieChartDataItem,
  MetricData,
} from "./types";
import {
  generateMockMetrics,
  generateMockBlockTimeData,
  generateMockBatchProcessData,
  generateMockSequencerData,
} from "./services/mockDataService";
import { fetchAvgProveTime, fetchAvgVerifyTime } from "./services/apiService";

const TAΙΚΟ_PINK = "#e81899"; // Updated Taiko Pink

const App: React.FC = () => {
  const [timeRange, setTimeRange] = useState<TimeRange>("1h");
  const [metrics, setMetrics] = useState<MetricData[]>([]);
  const [secondsToProveData, setSecondsToProveData] = useState<
    TimeSeriesData[]
  >([]);
  const [secondsToVerifyData, setSecondsToVerifyData] = useState<
    TimeSeriesData[]
  >([]);
  const [l2BlockTimeData, setL2BlockTimeData] = useState<TimeSeriesData[]>([]);
  const [l1BlockTimeData, setL1BlockTimeData] = useState<TimeSeriesData[]>([]);
  const [sequencerDistribution, setSequencerDistribution] = useState<
    PieChartDataItem[]
  >([]);
  const [l2HeadBlock, setL2HeadBlock] = useState<string>("0");
  const [l1HeadBlock, setL1HeadBlock] = useState<string>("0");
  const [refreshRate, setRefreshRate] = useState<number>(60000);

  const fetchData = useCallback(async () => {
    const currentMetrics = generateMockMetrics(timeRange);

    if (timeRange === "1h") {
      const [avgProve, avgVerify] = await Promise.all([
        fetchAvgProveTime(),
        fetchAvgVerifyTime(),
      ]);

      currentMetrics.forEach((m) => {
        if (m.title === "Avg. Prove Time" && avgProve !== null) {
          m.value = `${(avgProve / 1000).toFixed(2)}s`;
        }
        if (m.title === "Avg. Verify Time" && avgVerify !== null) {
          m.value = `${(avgVerify / 1000).toFixed(2)}s`;
        }
      });
    }

    setMetrics(currentMetrics);
    setSecondsToProveData(generateMockBatchProcessData(timeRange, "prove"));
    setSecondsToVerifyData(generateMockBatchProcessData(timeRange, "verify"));
    setL2BlockTimeData(generateMockBlockTimeData(timeRange, "L2"));
    setL1BlockTimeData(generateMockBlockTimeData(timeRange, "L1"));
    setSequencerDistribution(generateMockSequencerData());

    const l2BlockMetric = currentMetrics.find((m) =>
      m.title.includes("L2 Head Block"),
    );
    setL2HeadBlock(l2BlockMetric ? l2BlockMetric.value : "N/A");
    const l1BlockMetric = currentMetrics.find((m) =>
      m.title.includes("L1 Head Block"),
    );
    setL1HeadBlock(l1BlockMetric ? l1BlockMetric.value : "N/A");
  }, [timeRange]);

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, Math.max(refreshRate, 10000));
    return () => clearInterval(interval);
  }, [timeRange, fetchData, refreshRate]);

  const findMetricValue = (titlePart: string): string => {
    const metric = metrics.find((m) =>
      m.title.toLowerCase().includes(titlePart.toLowerCase()),
    );
    return metric ? metric.value : "N/A";
  };

  return (
    <div
      className="min-h-screen bg-white text-gray-800 p-4 md:p-6 lg:p-8"
      style={{ fontFamily: "'Inter', sans-serif" }}
    >
      <DashboardHeader
        timeRange={timeRange}
        onTimeRangeChange={setTimeRange}
        refreshRate={refreshRate}
        onRefreshRateChange={setRefreshRate}
      />

      <main className="mt-6">
        {/* Metrics Grid */}
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 md:gap-6">
          {/* Grouped Metrics */}
          <MetricCard
            title="L2 Block Cadence"
            value={findMetricValue("L2 Block Cadence")}
          />
          <MetricCard
            title="Batch Posting Cadence"
            value={findMetricValue("Batch Posting Cadence")}
          />
          <MetricCard
            title="Avg. Prove Time"
            value={findMetricValue("Avg. Prove Time")}
          />
          <MetricCard
            title="Avg. Verify Time"
            value={findMetricValue("Avg. Verify Time")}
          />

          {/* Other Metrics */}
          <MetricCard
            title="Active Gateways"
            value={findMetricValue("Active Gateways")}
          />
          <MetricCard title="L2 Reorgs" value={findMetricValue("L2 Reorgs")} />
          <MetricCard
            title="Slashing Events"
            value={findMetricValue("Slashing Events")}
          />
          <MetricCard
            title="Forced Inclusions"
            value={findMetricValue("Forced Inclusions")}
          />
        </div>

        {/* Charts Grid - Reordered: Sequencer Pie Chart first */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 md:gap-6 mt-6">
          <ChartCard title="Sequencer Distribution">
            <SequencerPieChart data={sequencerDistribution} />
          </ChartCard>
          <ChartCard title="Prove Time">
            <BatchProcessChart
              data={secondsToProveData}
              lineColor={TAΙΚΟ_PINK}
            />
          </ChartCard>
          <ChartCard title="Verify Time">
            <BatchProcessChart data={secondsToVerifyData} lineColor="#5DA5DA" />
          </ChartCard>
          <ChartCard title="L2 Block Times">
            <BlockTimeChart data={l2BlockTimeData} lineColor="#FAA43A" />
          </ChartCard>
          <ChartCard title="L1 Block Times">
            <BlockTimeChart data={l1BlockTimeData} lineColor="#60BD68" />
          </ChartCard>
        </div>
      </main>

      {/* Footer for Block Numbers */}
      <footer className="mt-8 pt-6 border-t border-gray-200">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-center md:text-left">
          <div>
            <span className="text-sm text-gray-500">L2 Head Block</span>
            <p className="text-2xl font-semibold" style={{ color: TAΙΚΟ_PINK }}>
              {l2HeadBlock}
            </p>
          </div>
          <div>
            <span className="text-sm text-gray-500">L1 Head Block</span>
            <p className="text-2xl font-semibold" style={{ color: TAΙΚΟ_PINK }}>
              {l1HeadBlock}
            </p>
          </div>
        </div>
        {/* Copyright notice removed as per request */}
      </footer>
    </div>
  );
};

export default App;
