import React, { useCallback, lazy, useState } from 'react';
import { ErrorDisplay } from '../layout/ErrorDisplay';
import { MetricsGrid } from '../layout/MetricsGrid';
import { ProfitCalculator } from '../ProfitCalculator';
import { IncomeChart } from '../IncomeChart';
import { CostChart } from '../CostChart';
import { ProfitabilityChart } from '../ProfitabilityChart';
import { ProfitRankingTable } from '../ProfitRankingTable';
import { FeeFlowChart } from '../FeeFlowChart';
import { ChartCard } from '../ChartCard';
import { TAIKO_PINK } from '../../theme';
import { TimeRange, MetricData } from '../../types';
import { useNavigate, useSearchParams } from 'react-router-dom';

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
  chartsData: any;

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
  const isEconomicsView = searchParams.get('view') === 'economics';
  // Default monthly costs in USD
  const [cloudCost, setCloudCost] = useState(1000);
  const [proverCost, setProverCost] = useState(1000);

  const visibleMetrics = React.useMemo(
    () =>
      metricsData.metrics.filter((m) => {
        if (selectedSequencer && m.group === 'Sequencers') return false;
        if (isEconomicsView) return m.group === 'Network Economics';
        return m.group !== 'Network Economics';
      }),
    [metricsData.metrics, selectedSequencer, isEconomicsView],
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
    : ['Network Performance', 'Network Health', 'Sequencers', 'Other'];

  const skeletonGroupCounts: Record<string, number> = isEconomicsView
    ? { 'Network Economics': 3 }
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
        'Avg. Prove Time': () => onOpenTable('prove-time', timeRange),
        'Avg. Verify Time': () => onOpenTable('verify-time', timeRange),
        'L1 Data Cost': () => onOpenTable('l1-data-cost', timeRange),
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
        onMore={() => onOpenTable('prove-time', timeRange)}
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
        onMore={() => onOpenTable('verify-time', timeRange)}
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
        onMore={() => onOpenTable('l2-block-times', timeRange)}
        loading={isLoadingData}
      >
        <BlockTimeDistributionChart
          key={`${timeRange}-d`}
          data={chartsData.l2BlockTimeData}
          barColor="#FAA43A"
        />
      </ChartCard>,
    ];

    const groups: Record<string, React.ReactNode[]> = {
      'Network Performance': performance,
      'Network Health': health,
    };

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
            <FeeFlowChart
              timeRange={timeRange}
              cloudCost={cloudCost}
              proverCost={proverCost}
              address={selectedSequencer || undefined}
            />
            <ProfitCalculator
              metrics={metricsData.metrics}
              timeRange={timeRange}
              cloudCost={cloudCost}
              proverCost={proverCost}
              onCloudCostChange={setCloudCost}
              onProverCostChange={setProverCost}
            />
            <div className="mt-6 grid grid-cols-1 md:grid-cols-2 gap-4 md:gap-6">
              <div>
                <IncomeChart
                  timeRange={timeRange}
                  address={selectedSequencer || undefined}
                />
              </div>
              <div>
                <CostChart
                  timeRange={timeRange}
                  cloudCost={cloudCost}
                  proverCost={proverCost}
                  address={selectedSequencer || undefined}
                />
              </div>
              <div>
                <ProfitabilityChart
                  timeRange={timeRange}
                  cloudCost={cloudCost}
                  proverCost={proverCost}
                  address={selectedSequencer || undefined}
                />
              </div>
            </div>
            <ProfitRankingTable
              timeRange={timeRange}
              cloudCost={cloudCost}
              proverCost={proverCost}
            />
          </>
        )}

        {/* Charts are now displayed within MetricsGrid groups */}
      </main>
    </div>
  );
};
