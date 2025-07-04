import React, { useCallback, lazy, useState } from 'react';
import { ErrorDisplay } from '../layout/ErrorDisplay';
import { MetricsGrid } from '../layout/MetricsGrid';
import { ProfitCalculator } from '../ProfitCalculator';
import { EconomicsChart } from '../EconomicsChart';
import { ProfitRankingTable } from '../ProfitRankingTable';
import { BlockProfitTables } from '../BlockProfitTables';
import { FeeFlowChart } from '../FeeFlowChart';
import { ChartCard } from '../ChartCard';
import { TAIKO_PINK } from '../../theme';
import { TimeRange, MetricData, ChartsData } from '../../types';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useEthPrice } from '../../services/priceService';
import { rangeToHours } from '../../utils/timeRange';
import { formatEth, parseEthValue } from '../../utils';

const SequencerPieChart = lazy(() =>
  import('../SequencerPieChart').then((m) => ({
    default: m.SequencerPieChart,
  })),
);
const BlockTimeDistributionChart = lazy(() =>
  import('../BlockTimeDistributionChart').then((m) => ({
    default: m.BlockTimeDistributionChart,
  })),
);
const BatchProcessChart = lazy(() =>
  import('../BatchProcessChart').then((m) => ({
    default: m.BatchProcessChart,
  })),
);
const GasUsedChart = lazy(() =>
  import('../GasUsedChart').then((m) => ({
    default: m.GasUsedChart,
  })),
);
const BlockTxChart = lazy(() =>
  import('../BlockTxChart').then((m) => ({
    default: m.BlockTxChart,
  })),
);
const BlobsPerBatchChart = lazy(() =>
  import('../BlobsPerBatchChart').then((m) => ({
    default: m.BlobsPerBatchChart,
  })),
);

interface DashboardViewProps {
  timeRange: TimeRange;
  selectedSequencer: string | null;

  // Data hooks
  metricsData: {
    metrics: MetricData[];
    loadingMetrics: boolean;
    errorMessage: string;
    setErrorMessage: (msg: string) => void;
  };
  chartsData: ChartsData;

  // Loading states
  isLoadingData: boolean;
  isTimeRangeChanging: boolean;

  // Actions
  onOpenTable: (table: string, timeRange?: TimeRange) => void;
  onOpenTpsTable: () => void;
  onOpenSequencerDistributionTable: (
    timeRange: TimeRange,
    page: number,
  ) => void;
}

export const DashboardView: React.FC<DashboardViewProps> = ({
  timeRange,
  selectedSequencer,
  metricsData,
  chartsData,
  isLoadingData,
  isTimeRangeChanging,
  onOpenTable,
  onOpenTpsTable,
  onOpenSequencerDistributionTable,
}) => {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const viewParam = searchParams.get('view') || 'economics';
  const isEconomicsView = viewParam === 'economics';
  const isPerformanceView = viewParam === 'performance';
  const isHealthView = viewParam === 'health';
  // Default monthly costs in USD
  const [cloudCost, setCloudCost] = useState(0);
  const [proverCost, setProverCost] = useState(0);

  const { data: ethPrice = 0 } = useEthPrice();
  const metricsWithHardware = React.useMemo(() => {
    if (!isEconomicsView) return metricsData.metrics;
    const HOURS_IN_MONTH = 30 * 24;
    const hours = rangeToHours(timeRange);
    const sequencerCount = chartsData.sequencerDistribution.length || 1;
    const costUsd =
      (((cloudCost + proverCost) * sequencerCount) / HOURS_IN_MONTH) * hours;
    const costWei = ethPrice > 0 ? (costUsd / ethPrice) * 1e9 : null;
    const hardwareMetric: MetricData = {
      title: 'Hardware Costs',
      value: costWei != null ? formatEth(costWei, 4) : 'N/A',
      group: 'Network Economics',
    };
    const idx = metricsData.metrics.findIndex((m) => m.title === 'Prove Cost');
    const list = [...metricsData.metrics];
    if (idx >= 0) list.splice(idx + 1, 0, hardwareMetric);
    else list.push(hardwareMetric);

    if (costWei != null) {
      const profitIdx = list.findIndex(
        (m) => m.title === 'Total Sequencer Profit',
      );
      if (profitIdx >= 0) {
        const profitEth = parseEthValue(list[profitIdx].value);
        const profitWei = profitEth * 1e9;
        const newProfitWei = profitWei - costWei;
        list[profitIdx] = {
          ...list[profitIdx],
          value: formatEth(newProfitWei, 4),
        };
      }
    }

    return list;
  }, [
    metricsData.metrics,
    isEconomicsView,
    cloudCost,
    proverCost,
    ethPrice,
    timeRange,
  ]);

  const visibleMetrics = React.useMemo(
    () =>
      metricsWithHardware.filter((m) => {
        if (selectedSequencer && m.group === 'Sequencers') return false;
        if (isEconomicsView) return m.group === 'Network Economics';
        if (isPerformanceView) return m.group === 'Network Performance';
        if (isHealthView) return m.group === 'Network Health';
        return m.group !== 'Network Economics';
      }),
    [
      metricsWithHardware,
      selectedSequencer,
      isEconomicsView,
      isPerformanceView,
      isHealthView,
    ],
  );

  const groupedMetrics = React.useMemo(
    () =>
      visibleMetrics.reduce<Record<string, MetricData[]>>((acc, m) => {
        const group = m.group ?? 'Other';
        if (!acc[group]) acc[group] = [];
        acc[group].push(m);
        return acc;
      }, {}),
    [visibleMetrics],
  );

  const groupOrder = isEconomicsView
    ? ['Network Economics']
    : isPerformanceView
      ? ['Network Performance', 'Sequencers', 'Other']
      : isHealthView
        ? ['Network Health', 'Sequencers', 'Other']
        : ['Network Performance', 'Network Health', 'Sequencers', 'Other'];

  const skeletonGroupCounts: Record<string, number> = isEconomicsView
    ? { 'Network Economics': 6 }
    : isPerformanceView
      ? {
          'Network Performance': 3,
          Sequencers: 3,
        }
      : isHealthView
        ? {
            'Network Health': 5,
            Sequencers: 3,
          }
        : {
            'Network Performance': 3,
            'Network Health': 5,
            Sequencers: 3,
          };

  const displayGroupName = useCallback(
    (group: string): string => {
      if (!selectedSequencer) return group;
      if (group === 'Network Performance') return 'Sequencer Performance';
      if (group === 'Network Health') return 'Sequencer Health';
      return group;
    },
    [selectedSequencer],
  );

  const displayedGroupOrder = selectedSequencer
    ? groupOrder.filter((g) => g !== 'Sequencers')
    : groupOrder;

  const handleResetNavigation = useCallback(() => {
    navigate('/', { replace: true });
    metricsData.setErrorMessage('');
  }, [navigate, metricsData]);

  const handleClearError = useCallback(() => {
    metricsData.setErrorMessage('');
  }, [metricsData]);

  const getMetricAction = useCallback(
    (title: string) => {
      const actions: Record<string, () => void> = {
        'Avg. L2 TPS': onOpenTpsTable,
        'L2 Reorgs': () => onOpenTable('reorgs'),
        'Slashing Events': () => onOpenTable('slashings'),
        'Forced Inclusions': () => onOpenTable('forced-inclusions'),
        'Active Sequencers': () => onOpenTable('gateways'),
        'Batch Posting Cadence': () => onOpenTable('batch-posting-cadence'),
        'Avg. Prove Time': () => onOpenTable('prove-times', timeRange),
        'Avg. Verify Time': () => onOpenTable('verify-times', timeRange),
        'Propose Batch Cost': () => onOpenTable('l1-data-cost', timeRange),
        'Prove Cost': () => onOpenTable('prove-cost', timeRange),
      };
      return actions[title];
    },
    [onOpenTable, onOpenTpsTable, timeRange],
  );

  const groupedCharts = React.useMemo(() => {
    if (isEconomicsView) return {} as Record<string, React.ReactNode[]>;

    const performance = [
      <ChartCard
        key="gas"
        title="Avg Gas Used Per Block"
        onMore={() => onOpenTable('l2-gas-used', timeRange)}
        loading={isLoadingData}
      >
        <GasUsedChart
          key={`${timeRange}-g`}
          data={chartsData.l2GasUsedData}
          lineColor="#E573B5"
        />
      </ChartCard>,
      <ChartCard
        key="tx"
        title="Avg Tx Count Per L2 Block"
        onMore={() => onOpenTable('block-tx', timeRange)}
        loading={isLoadingData}
      >
        <BlockTxChart
          key={`${timeRange}-t`}
          data={chartsData.blockTxData}
          lineColor="#4E79A7"
        />
      </ChartCard>,
    ];

    const health = [
      <ChartCard
        key="prove"
        title="Avg Prove Time"
        onMore={() => onOpenTable('prove-times', timeRange)}
        loading={isLoadingData}
      >
        <BatchProcessChart
          key={timeRange}
          data={chartsData.secondsToProveData}
          lineColor={TAIKO_PINK}
        />
      </ChartCard>,
      <ChartCard
        key="verify"
        title="Avg Verify Time"
        onMore={() => onOpenTable('verify-times', timeRange)}
        loading={isLoadingData}
      >
        <BatchProcessChart
          key={`${timeRange}-v`}
          data={chartsData.secondsToVerifyData}
          lineColor="#5DA5DA"
        />
      </ChartCard>,
      <ChartCard
        key="blobs"
        title="Avg Blobs per Batch"
        onMore={() => onOpenTable('blobs-per-batch', timeRange)}
        loading={isLoadingData}
      >
        <BlobsPerBatchChart
          key={`${timeRange}-b`}
          data={chartsData.batchBlobCounts}
          barColor="#A0CBE8"
        />
      </ChartCard>,
      <ChartCard
        key="block-times"
        title="L2 Block Time Distribution"
        loading={isLoadingData}
      >
        <BlockTimeDistributionChart
          key={`${timeRange}-d`}
          data={chartsData.l2BlockTimeData}
          barColor="#FAA43A"
        />
      </ChartCard>,
    ];

    const groups: Record<string, React.ReactNode[]> = {};
    if (isPerformanceView || (!isPerformanceView && !isHealthView)) {
      groups['Network Performance'] = performance;
    }
    if (isHealthView || (!isPerformanceView && !isHealthView)) {
      groups['Network Health'] = health;
    }

    if (!selectedSequencer) {
      groups['Sequencers'] = [
        <ChartCard
          key="seq-dist"
          title="Sequencer Distribution"
          onMore={() => onOpenSequencerDistributionTable(timeRange, 0)}
          loading={isLoadingData}
        >
          <SequencerPieChart
            key={`${timeRange}-s`}
            data={chartsData.sequencerDistribution}
          />
        </ChartCard>,
      ];
    }

    return groups;
  }, [
    chartsData,
    timeRange,
    selectedSequencer,
    isLoadingData,
    isEconomicsView,
    isPerformanceView,
    isHealthView,
    onOpenTable,
    onOpenSequencerDistributionTable,
  ]);

  return (
    <div
      className="bg-white dark:bg-gray-900 text-gray-800 dark:text-gray-100 p-4 md:p-6 lg:p-8"
      style={{ fontFamily: "'Inter', sans-serif" }}
    >
      <ErrorDisplay
        errorMessage={metricsData.errorMessage}
        onResetNavigation={handleResetNavigation}
        onClearError={handleClearError}
      />

      <main className="mt-6">
        <MetricsGrid
          isLoading={metricsData.loadingMetrics}
          groupedMetrics={groupedMetrics}
          groupOrder={displayedGroupOrder}
          skeletonGroupCounts={skeletonGroupCounts}
          displayGroupName={displayGroupName}
          onMetricAction={getMetricAction}
          economicsView={isEconomicsView}
          groupedCharts={groupedCharts}
          isTimeRangeChanging={isTimeRangeChanging}
          timeRange={timeRange}
        />

        {isEconomicsView && (
          <>
            <ProfitCalculator
              timeRange={timeRange}
              cloudCost={cloudCost}
              proverCost={proverCost}
              onCloudCostChange={setCloudCost}
              onProverCostChange={setProverCost}
            />
            <FeeFlowChart
              timeRange={timeRange}
              cloudCost={cloudCost}
              proverCost={proverCost}
              address={selectedSequencer || undefined}
              height={400}
              totalSequencers={chartsData.sequencerDistribution.length}
            />
            <div className="mt-6">
              <h3 className="text-lg font-semibold mb-2">
                PnL Trend per Batch
              </h3>
              <EconomicsChart
                timeRange={timeRange}
                cloudCost={cloudCost}
                proverCost={proverCost}
                address={selectedSequencer || undefined}
              />
            </div>
            <ProfitRankingTable
              timeRange={timeRange}
              cloudCost={cloudCost}
              proverCost={proverCost}
            />
            <BlockProfitTables
              timeRange={timeRange}
              cloudCost={cloudCost}
              proverCost={proverCost}
              address={selectedSequencer || undefined}
            />
          </>
        )}

        {/* Charts are now displayed within MetricsGrid groups */}
      </main>
    </div>
  );
};
