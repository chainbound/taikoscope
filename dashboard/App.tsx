import React from 'react';
import { useDashboardController } from './hooks/useDashboardController';
import { DashboardView } from './components/views/DashboardView';
import { TableView } from './components/views/TableView';

const App: React.FC = () => {
  const {
    // State
    timeRange,
    setTimeRange,
    selectedSequencer,
    sequencerList,

    // Data
    metricsData,
    chartsData,
    blockData,
    refreshTimer,
    searchParams,

    // Table state
    tableView,
    tableLoading,

    // Handlers
    handleSequencerChangeWithState,
    handleBack,
    handleManualRefresh,

    // Table actions
    openGenericTable,
    openTpsTable,
    openSequencerDistributionTable,
  } = useDashboardController();

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
