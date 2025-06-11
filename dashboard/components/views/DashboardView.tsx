import React, { useCallback } from 'react';
import { ErrorDisplay } from '../layout/ErrorDisplay';
import { MetricsGrid } from '../layout/MetricsGrid';
import { ChartsGrid } from '../layout/ChartsGrid';
import { ProfitCalculator } from '../ProfitCalculator';
import { TimeRange, MetricData } from '../../types';
import { useNavigate, useSearchParams } from 'react-router-dom';

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
        <MetricsGrid
          isLoading={metricsData.loadingMetrics}
          groupedMetrics={groupedMetrics}
          groupOrder={displayedGroupOrder}
          skeletonGroupCounts={skeletonGroupCounts}
          displayGroupName={displayGroupName}
          onMetricAction={getMetricAction}
          economicsView={isEconomicsView}
        />

        {isEconomicsView && (
          <ProfitCalculator metrics={metricsData.metrics} timeRange={timeRange} />
        )}

        {!isEconomicsView && (
          <ChartsGrid
            isLoading={isLoadingData}
            isTimeRangeChanging={isTimeRangeChanging}
            timeRange={timeRange}
            selectedSequencer={selectedSequencer}
            chartsData={chartsData}
            onOpenTable={onOpenTable}
            onOpenSequencerDistributionTable={onOpenSequencerDistributionTable}
          />
        )}
      </main>
    </div>
  );
};
