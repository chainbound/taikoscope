import React from 'react';
import { Outlet } from 'react-router-dom';
import { DashboardHeader } from '../DashboardHeader';
import { useTimeRangeSync } from '../../hooks/useTimeRangeSync';
import { useSequencerHandler } from '../../hooks/useSequencerHandler';
import { useChartsData } from '../../hooks/useChartsData';
import { useBlockData } from '../../hooks/useBlockData';
import { useMetricsData } from '../../hooks/useMetricsData';

export const DashboardLayout: React.FC = () => {
  const { timeRange, setTimeRange } = useTimeRangeSync();
  const chartsData = useChartsData();
  const blockData = useBlockData();
  const metricsData = useMetricsData();
  
  const { selectedSequencer, setSelectedSequencer, sequencerList } = useSequencerHandler({
    chartsData,
    blockData,
    metricsData,
  });

  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-900">
      <DashboardHeader
        timeRange={timeRange}
        onTimeRangeChange={setTimeRange}
        refreshRate={60000}
        onRefreshRateChange={() => {}}
        lastRefresh={Date.now()}
        onManualRefresh={() => {}}
        sequencers={sequencerList}
        selectedSequencer={selectedSequencer}
        onSequencerChange={setSelectedSequencer}
      />
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
        }} />
      </main>
    </div>
  );
};