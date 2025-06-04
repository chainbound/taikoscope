import React, { useState } from 'react';
import { useMetricsData } from './hooks/useMetricsData';
import { useChartsData } from './hooks/useChartsData';
import { useBlockData } from './hooks/useBlockData';
import { useRefreshTimer } from './hooks/useRefreshTimer';
import { useTableRouter } from './hooks/useTableRouter';
import { useNavigationHandler } from './hooks/useNavigationHandler';
import { useDataFetcher } from './hooks/useDataFetcher';
import { useSequencerHandler } from './hooks/useSequencerHandler';
import { DashboardView } from './components/views/DashboardView';
import { TableView } from './components/views/TableView';
import { useTableActions } from './hooks/useTableActions';
import { useSearchParams } from './hooks/useSearchParams';
import {
  TimeRange,
} from './types';

const App: React.FC = () => {
  const [timeRange, setTimeRange] = useState<TimeRange>('1h');
  const searchParams = useSearchParams();

  // Data management hooks
  const metricsData = useMetricsData();
  const chartsData = useChartsData();
  const blockData = useBlockData();
  const refreshTimer = useRefreshTimer();

  // Sequencer handling
  const { selectedSequencer, setSelectedSequencer, sequencerList } = useSequencerHandler({
    chartsData,
    blockData,
    metricsData,
  });

  // Table actions
  const {
    tableView,
    tableLoading,
    setTableView,
    setTableLoading,
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

  // Data fetching coordination
  const { handleManualRefresh } = useDataFetcher({
    timeRange,
    selectedSequencer,
    tableView,
    metricsData,
    chartsData,
    refreshTimer,
  });

  // Navigation handling
  const { handleBack, handleSequencerChange } = useNavigationHandler({
    setTableView,
    onError: metricsData.setErrorMessage,
  });

  // Table routing
  useTableRouter({
    timeRange,
    setTableView,
    setTableLoading,
    tableView,
    openGenericTable,
    openTpsTable,
    openSequencerDistributionTable,
    onError: metricsData.setErrorMessage,
  });

  // Combined sequencer change handler
  const handleSequencerChangeWithState = (seq: string | null) => {
    setSelectedSequencer(seq);
    handleSequencerChange(seq);
  };

  if (tableView) {
    return (
      <TableView
        tableView={tableView}
        tableLoading={tableLoading}
        isNavigating={searchParams.navigationState.isNavigating}
        refreshTimer={refreshTimer}
        sequencerList={sequencerList}
        selectedSequencer={selectedSequencer}
        onSequencerChange={handleSequencerChangeWithState}
        onBack={handleBack}
        onManualRefresh={handleManualRefresh}
      />
    );
  }

  return (
    <DashboardView
      timeRange={timeRange}
      onTimeRangeChange={setTimeRange}
      selectedSequencer={selectedSequencer}
      onSequencerChange={handleSequencerChangeWithState}
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

export default App;
