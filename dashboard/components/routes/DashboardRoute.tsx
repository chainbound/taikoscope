import React from 'react';
import { useOutletContext } from 'react-router-dom';
import { DashboardView } from '../views/DashboardView';
import { useRefreshTimer } from '../../hooks/useRefreshTimer';
import { useDataFetcher } from '../../hooks/useDataFetcher';
import { useTableActions } from '../../hooks/useTableActions';
import { TimeRange } from '../../types';

interface DashboardContextType {
  timeRange: TimeRange;
  setTimeRange: (range: TimeRange) => void;
  selectedSequencer: string | null;
  setSelectedSequencer: (seq: string | null) => void;
  sequencerList: string[];
  chartsData: any;
  blockData: any;
  metricsData: any;
}

export const DashboardRoute: React.FC = () => {
  const {
    timeRange,
    setTimeRange,
    selectedSequencer,
    setSelectedSequencer,
    sequencerList,
    chartsData,
    blockData,
    metricsData,
  } = useOutletContext<DashboardContextType>();

  const refreshTimer = useRefreshTimer();

  const {
    openGenericTable,
    openTpsTable,
    openSequencerDistributionTable,
  } = useTableActions(
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
    <DashboardView
      timeRange={timeRange}
      onTimeRangeChange={setTimeRange}
      selectedSequencer={selectedSequencer}
      onSequencerChange={setSelectedSequencer}
      sequencerList={sequencerList}
      metricsData={metricsData}
      chartsData={chartsData}
      blockData={blockData}
      refreshTimer={refreshTimer}
      onManualRefresh={handleManualRefresh}
      onOpenTable={openGenericTable}
      onOpenTpsTable={openTpsTable}
      onOpenSequencerDistributionTable={openSequencerDistributionTable}
    />
  );
};