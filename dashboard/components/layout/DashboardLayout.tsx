import React from 'react';
import { Outlet } from 'react-router-dom';
import { useTimeRangeSync } from '../../hooks/useTimeRangeSync';
import { useSequencerHandler } from '../../hooks/useSequencerHandler';
import { useChartsData } from '../../hooks/useChartsData';
import { useBlockData } from '../../hooks/useBlockData';
import { useMetricsData } from '../../hooks/useMetricsData';
import { useRefreshTimer } from '../../hooks/useRefreshTimer';

export const DashboardLayout: React.FC = () => {
  const { timeRange, setTimeRange } = useTimeRangeSync();
  const chartsData = useChartsData();
  const blockData = useBlockData();
  const metricsData = useMetricsData();
  const refreshTimer = useRefreshTimer();
  
  const { selectedSequencer, setSelectedSequencer, sequencerList } = useSequencerHandler({
    chartsData,
    blockData,
    metricsData,
  });

  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-900">
      <main className="container mx-auto px-4 py-6">
        <Outlet context={{
          timeRange,
          setTimeRange,
          selectedSequencer,
          setSelectedSequencer,
          sequencerList,
          chartsData,
          blockData,
          metricsData,
          refreshTimer,
        }} />
      </main>
    </div>
  );
};