import React, { useEffect } from 'react';
import { useParams, useOutletContext } from 'react-router-dom';
import { useRouterNavigation } from '../../hooks/useRouterNavigation';
import {
  TimeRange,
  ChartsData,
  BlockDataState,
  MetricsDataState,
  RefreshTimerState,
} from '../../types';

interface DashboardContextType {
  timeRange: TimeRange;
  setTimeRange: (range: TimeRange) => void;
  selectedSequencer: string | null;
  setSelectedSequencer: (seq: string | null) => void;
  sequencerList: string[];
  chartsData: ChartsData;
  blockData: BlockDataState;
  metricsData: MetricsDataState;
  refreshTimer: RefreshTimerState;
}

export const SequencerRoute: React.FC = () => {
  const { address } = useParams<{ address: string }>();
  const { navigateToTable, navigateToDashboard } = useRouterNavigation();

  const { setSelectedSequencer, sequencerList, timeRange } =
    useOutletContext<DashboardContextType>();

  useEffect(() => {
    if (address && sequencerList.includes(address)) {
      setSelectedSequencer(address);
      navigateToTable('sequencer-blocks', { address }, timeRange);
    } else {
      navigateToDashboard();
    }
  }, [
    address,
    sequencerList,
    setSelectedSequencer,
    navigateToTable,
    navigateToDashboard,
    timeRange,
  ]);

  return <div>Redirecting...</div>;
};
