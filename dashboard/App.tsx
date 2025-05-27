import React, { useState, useEffect, useCallback } from "react";
import { formatSeconds } from "./utils";
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
  API_BASE,
  fetchAvgProveTime,
  fetchAvgVerifyTime,
  fetchL2BlockCadence,
  fetchBatchPostingCadence,
  fetchActiveGateways,
  fetchL2Reorgs,
  fetchSlashingEvents,
  fetchForcedInclusions,
  fetchL2HeadBlock,
  fetchL1HeadBlock,
  fetchProveTimes,
  fetchVerifyTimes,
  fetchL1BlockTimes,
  fetchL2BlockTimes,
  fetchSequencerDistribution,
} from "./services/apiService";

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
  const [errorMessage, setErrorMessage] = useState<string>("");

  useEffect(() => {
    const l1Source = new EventSource(`${API_BASE}/sse/l1-head`);
    const l2Source = new EventSource(`${API_BASE}/sse/l2-head`);

    l1Source.onmessage = (e) => {
      const value = Number(e.data).toLocaleString();
      setL1HeadBlock(value);
      setMetrics((m) =>
        m.map((metric) =>
          metric.title === "L1 Head Block" ? { ...metric, value } : metric,
        ),
      );
    };
    l2Source.onmessage = (e) => {
      const value = Number(e.data).toLocaleString();
      setL2HeadBlock(value);
      setMetrics((m) =>
        m.map((metric) =>
          metric.title === "L2 Head Block" ? { ...metric, value } : metric,
        ),
      );
    };

    return () => {
      l1Source.close();
      l2Source.close();
    };
  }, []);

  const fetchData = useCallback(async () => {
    const range = timeRange;
    const [
      l2CadenceRes,
      batchCadenceRes,
      avgProveRes,
      avgVerifyRes,
      activeGatewaysRes,
      l2ReorgsRes,
      slashingsRes,
      forcedInclusionsRes,
      l2BlockRes,
      l1BlockRes,
      proveTimesRes,
      verifyTimesRes,
      l1TimesRes,
      l2TimesRes,
      sequencerDistRes,
    ] = await Promise.all([
      fetchL2BlockCadence(range),
      fetchBatchPostingCadence(range),
      fetchAvgProveTime(range),
      fetchAvgVerifyTime(range),
      fetchActiveGateways(range),
      fetchL2Reorgs(range),
      fetchSlashingEvents(range),
      fetchForcedInclusions(range),
      fetchL2HeadBlock(range),
      fetchL1HeadBlock(range),
      fetchProveTimes(range),
      fetchVerifyTimes(range),
      fetchL1BlockTimes(range),
      fetchL2BlockTimes(range),
      fetchSequencerDistribution(range),
    ]);

    const l2Cadence = l2CadenceRes.data;
    const batchCadence = batchCadenceRes.data;
    const avgProve = avgProveRes.data;
    const avgVerify = avgVerifyRes.data;
    const activeGateways = activeGatewaysRes.data;
    const l2Reorgs = l2ReorgsRes.data;
    const slashings = slashingsRes.data;
    const forcedInclusions = forcedInclusionsRes.data;
    const l2Block = l2BlockRes.data;
    const l1Block = l1BlockRes.data;
    const proveTimes = proveTimesRes.data || [];
    const verifyTimes = verifyTimesRes.data || [];
    const l1Times = l1TimesRes.data || [];
    const l2Times = l2TimesRes.data || [];
    const sequencerDist = sequencerDistRes.data || [];

    const anyBadRequest =
      l2CadenceRes.badRequest ||
      batchCadenceRes.badRequest ||
      avgProveRes.badRequest ||
      avgVerifyRes.badRequest ||
      activeGatewaysRes.badRequest ||
      l2ReorgsRes.badRequest ||
      slashingsRes.badRequest ||
      forcedInclusionsRes.badRequest ||
      l2BlockRes.badRequest ||
      l1BlockRes.badRequest ||
      proveTimesRes.badRequest ||
      verifyTimesRes.badRequest ||
      l1TimesRes.badRequest ||
      l2TimesRes.badRequest ||
      sequencerDistRes.badRequest;

    const currentMetrics: MetricData[] = [
      {
        title: "L2 Block Cadence",
        value: l2Cadence !== null ? formatSeconds(l2Cadence / 1000) : "N/A",
      },
      {
        title: "Batch Posting Cadence",
        value:
          batchCadence !== null ? formatSeconds(batchCadence / 1000) : "N/A",
      },
      {
        title: "Avg. Prove Time",
        value:
          avgProve !== null && avgProve > 0
            ? formatSeconds(avgProve / 1000)
            : "N/A",
      },
      {
        title: (
          <a
            href="https://docs.taiko.xyz/taiko-alethia-protocol/protocol-architecture/block-states"
            target="_blank"
            rel="noopener noreferrer"
            className="hover:underline"
          >
            Avg. Verify Time
          </a>
        ),
        value:
          avgVerify !== null && avgVerify > 0
            ? formatSeconds(avgVerify / 1000)
            : "N/A",
      },
      {
        title: "Active Gateways",
        value: activeGateways !== null ? activeGateways.toString() : "N/A",
      },
      {
        title: "L2 Reorgs",
        value: l2Reorgs !== null ? l2Reorgs.toString() : "N/A",
      },
      {
        title: "Slashing Events",
        value: slashings !== null ? slashings.toString() : "N/A",
      },
      {
        title: "Forced Inclusions",
        value: forcedInclusions !== null ? forcedInclusions.toString() : "N/A",
      },
      {
        title: "L2 Head Block",
        value: l2Block !== null ? l2Block.toLocaleString() : "N/A",
      },
      {
        title: "L1 Head Block",
        value: l1Block !== null ? l1Block.toLocaleString() : "N/A",
      },
    ];

    setMetrics(currentMetrics);
    setSecondsToProveData(proveTimes);
    setSecondsToVerifyData(verifyTimes);
    setL2BlockTimeData(l2Times);
    setL1BlockTimeData(l1Times);
    setSequencerDistribution(sequencerDist);
    setL2HeadBlock(
      currentMetrics.find((m) => m.title === "L2 Head Block")?.value || "N/A",
    );
    setL1HeadBlock(
      currentMetrics.find((m) => m.title === "L1 Head Block")?.value || "N/A",
    );
    if (anyBadRequest) {
      setErrorMessage(
        "Invalid parameters provided. Some data may not be available.",
      );
    } else {
      setErrorMessage("");
    }
  }, [timeRange]);

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, Math.max(refreshRate, 10000));
    return () => clearInterval(interval);
  }, [timeRange, fetchData, refreshRate]);

  const findMetricValue = (titlePart: string): string => {
    const metric = metrics.find((m) => {
      const titleStr =
        typeof m.title === "string" ? m.title : "Avg. Verify Time";
      return titleStr.toLowerCase().includes(titlePart.toLowerCase());
    });
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

      {errorMessage && (
        <div className="mt-4 p-3 bg-red-50 border border-red-200 text-red-700 rounded">
          {errorMessage}
        </div>
      )}

      <main className="mt-6">
        {/* Metrics Grid */}
        <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-5 xl:grid-cols-6 2xl:grid-cols-8 gap-4 md:gap-6">
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
