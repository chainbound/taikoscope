import React from 'react';
import { useOutletContext } from 'react-router-dom';
import { DashboardView } from '../views/DashboardView';
import { DashboardHeader } from '../DashboardHeader';
import { useDataFetcher } from '../../hooks/useDataFetcher';
import { useTableActions } from '../../hooks/useTableActions';
import { TimeRange } from '../../types';

interface DashboardContextType {
  timeRange: TimeRange;
  setTimeRange: (range: TimeRange) => void;
  selectedSequencer: string | null;
  chartsData: any;
  metricsData: any;
  refreshTimer: any;
}

export const DashboardRoute: React.FC = () => {
  const {
    timeRange,
    setTimeRange,
    selectedSequencer,
    chartsData,
    metricsData,
    refreshTimer,
  } = useOutletContext<DashboardContextType>();

  const { openGenericTable, openTpsTable, openSequencerDistributionTable } =
    useTableActions(
      timeRange,
      setTimeRange,
      selectedSequencer,
      chartsData.blockTxData,
      chartsData.l2BlockTimeData,
    );

  const { handleManualRefresh } = useDataFetcher({
    timeRange,
    selectedSequencer,
    tableView: null,
    fetchMetricsData: metricsData.fetchMetricsData,
    updateChartsData: chartsData.updateChartsData,
    refreshRate: refreshTimer.refreshRate,
    updateLastRefresh: refreshTimer.updateLastRefresh,
  });

  return (
    <>
      <DashboardHeader
        timeRange={timeRange}
        onTimeRangeChange={setTimeRange}
        refreshRate={refreshTimer.refreshRate}
        onRefreshRateChange={refreshTimer.setRefreshRate}
        lastRefresh={refreshTimer.lastRefresh}
        onManualRefresh={handleManualRefresh}
      />
      <DashboardView
        timeRange={timeRange}
        selectedSequencer={selectedSequencer}
        metricsData={metricsData}
        chartsData={chartsData}
        onOpenTable={openGenericTable}
        onOpenTpsTable={openTpsTable}
        onOpenSequencerDistributionTable={openSequencerDistributionTable}
      />
    </>
  );
};
