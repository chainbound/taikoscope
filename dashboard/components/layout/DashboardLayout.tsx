import React, { useEffect } from 'react';
import { Outlet, useSearchParams } from 'react-router-dom';
import { DashboardFooter } from '../DashboardFooter';
import { useDashboardController } from '../../hooks/useDashboardController';
import { useRouterNavigation } from '../../hooks/useRouterNavigation';
import { DEFAULT_VIEW } from '../../constants';

export const DashboardLayout: React.FC = () => {
  const {
    timeRange,
    setTimeRange,
    selectedSequencer,
    sequencerList,
    metricsData,
    chartsData,
    blockData,
    refreshTimer,
    isLoadingData,
    isTimeRangeChanging,
    hasData,
    tableView,
    tableLoading,
    handleSequencerChangeWithState,
    handleBack,
    handleManualRefresh,
    openGenericTable,
    openTpsTable,
    openSequencerDistributionTable,
  } = useDashboardController();
  const { updateSearchParams } = useRouterNavigation();
  const [searchParams] = useSearchParams();

  useEffect(() => {
    if (!searchParams.get('view')) {
      updateSearchParams({ view: DEFAULT_VIEW });
    }
  }, [searchParams, updateSearchParams]);

  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-900 text-gray-900 dark:text-gray-100 flex flex-col">
      <main className="flex-grow px-4 py-6 md:px-6 lg:px-8">
        <Outlet
          context={{
            timeRange,
            setTimeRange,
            selectedSequencer,
            setSelectedSequencer: handleSequencerChangeWithState,
            sequencerList,
            chartsData,
            blockData,
            metricsData,
            refreshTimer,
            isLoadingData,
            isTimeRangeChanging,
            hasData,
            tableView,
            tableLoading,
            handleBack,
            handleManualRefresh,
            openGenericTable,
            openTpsTable,
            openSequencerDistributionTable,
          }}
        />
      </main>
      <DashboardFooter
        l2HeadBlock={blockData.l2HeadBlock}
        l1HeadBlock={blockData.l1HeadBlock}
      />
    </div>
  );
};
