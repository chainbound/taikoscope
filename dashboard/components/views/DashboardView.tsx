import React, { useCallback } from 'react';
import { ErrorDisplay } from '../layout/ErrorDisplay';
import { MetricsGrid } from '../layout/MetricsGrid';
import { ProfitCalculator } from '../ProfitCalculator';
import { ProfitabilityChart } from '../ProfitabilityChart';
import { TimeRange, MetricData } from '../../types';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { BlockTimeDistributionChart } from '../BlockTimeDistributionChart';
import { GasUsedChart } from '../GasUsedChart';
import { BatchProcessChart } from '../BatchProcessChart';
import { SequencerPieChart } from '../SequencerPieChart';
import { BlockTxChart } from '../BlockTxChart';
import { BlobsPerBatchChart } from '../BlobsPerBatchChart';
import { ChartCard } from '../ChartCard';
import { TAIKO_PINK } from '../../theme';

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
  onOpenTable,
  onOpenTpsTable,
  onOpenSequencerDistributionTable,
}) => {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const isEconomicsView = searchParams.get('view') === 'economics';
  const hoursMap: Record<TimeRange, number> = {
    '15m': 0.25,
    '1h': 1,
    '24h': 24,
  };

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
    ? { 'Network Economics': 1 }
    : {
      'Network Performance': 5,
      'Network Health': 4,
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

  const charts = [
    {
      group: 'Network Performance',
      title: 'L2 Block Time Distribution',
      onMore: () => onOpenTable('l2-block-times', timeRange),
      component: (
        <BlockTimeDistributionChart
          key={timeRange}
          data={chartsData.l2BlockTimeData}
          barColor="#FAA43A"
        />
      ),
    },
    {
      group: 'Network Performance',
      title: 'Gas Used Per Block',
      onMore: () => onOpenTable('l2-gas-used', timeRange),
      component: (
        <GasUsedChart
          key={timeRange}
          data={chartsData.l2GasUsedData}
          lineColor="#E573B5"
        />
      ),
    },
    {
      group: 'Network Health',
      title: 'Prove Time',
      onMore: () => onOpenTable('prove-time', timeRange),
      component: (
        <BatchProcessChart
          key={timeRange}
          data={chartsData.secondsToProveData}
          lineColor={TAIKO_PINK}
        />
      ),
    },
    {
      group: 'Network Health',
      title: 'Verify Time',
      onMore: () => onOpenTable('verify-time', timeRange),
      component: (
        <BatchProcessChart
          key={timeRange}
          data={chartsData.secondsToVerifyData}
          lineColor="#5DA5DA"
        />
      ),
    },
    {
      group: 'Sequencers',
      title: 'Sequencer Distribution',
      onMore: () => onOpenSequencerDistributionTable(timeRange, 0),
      component: (
        <SequencerPieChart
          key={timeRange}
          data={chartsData.sequencerDistribution}
        />
      ),
      hide: !!selectedSequencer,
    },
    {
      group: 'Other Metrics',
      title: 'Tx Count Per L2 Block',
      onMore: () => onOpenTable('block-tx', timeRange),
      component: (
        <BlockTxChart
          key={timeRange}
          data={chartsData.blockTxData}
          lineColor="#4E79A7"
        />
      ),
    },
    {
      group: 'Other Metrics',
      title: 'Blobs per Batch',
      onMore: () => onOpenTable('blobs-per-batch', timeRange),
      component: (
        <BlobsPerBatchChart
          key={timeRange}
          data={chartsData.batchBlobCounts}
          barColor="#A0CBE8"
        />
      ),
    },
  ];

  const renderChartsByGroup = (group: string) => (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 md:gap-6 mt-4">
      {charts
        .filter((c) => c.group === group && !c.hide)
        .map((chart, idx) => (
          <ChartCard
            key={idx}
            title={chart.title}
            onMore={chart.onMore}
            loading={isLoadingData}
          >
            {chart.component}
          </ChartCard>
        ))}
    </div>
  );

  const getMetricAction = useCallback(
    (title: string) => {
      const actions: Record<string, () => void> = {
        'Avg. L2 TPS': onOpenTpsTable,
        'L2 Reorgs': () => onOpenTable('reorgs'),
        'Slashing Events': () => onOpenTable('slashings'),
        'Forced Inclusions': () => onOpenTable('forced-inclusions'),
        'Active Sequencers': () => onOpenTable('gateways'),
        'Batch Posting Cadence': () => onOpenTable('batch-posting-cadence'),
      };
      return actions[title];
    },
    [onOpenTable, onOpenTpsTable],
  );

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
        {displayedGroupOrder.map((group) => (
          <React.Fragment key={group}>
            <MetricsGrid
              isLoading={metricsData.loadingMetrics}
              groupedMetrics={{ [group]: groupedMetrics[group] }}
              groupOrder={[group]}
              skeletonGroupCounts={skeletonGroupCounts}
              displayGroupName={displayGroupName}
              onMetricAction={getMetricAction}
              economicsView={isEconomicsView}
            />
            {renderChartsByGroup(group)}
          </React.Fragment>
        ))}

        {!isEconomicsView && (
          <>
            <h2 className="text-xl font-bold mt-6">Other Metrics</h2>
            {renderChartsByGroup('Other Metrics')}
          </>
        )}

        {isEconomicsView && (
          <>
            <ProfitCalculator
              metrics={metricsData.metrics}
              timeRange={timeRange}
            />
            <div className="mt-6">
              <ProfitabilityChart
                metrics={metricsData.metrics}
                hours={hoursMap[timeRange]}
              />
            </div>
          </>
        )}
      </main>
    </div>
  );
};
