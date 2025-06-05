import React, { useEffect } from 'react';
import { useParams, useOutletContext, useNavigate } from 'react-router-dom';
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
  refreshTimer: any;
}

export const SequencerRoute: React.FC = () => {
  const { address } = useParams<{ address: string }>();
  const navigate = useNavigate();
  
  const {
    setSelectedSequencer,
    sequencerList,
    timeRange,
  } = useOutletContext<DashboardContextType>();

  useEffect(() => {
    if (address && sequencerList.includes(address)) {
      setSelectedSequencer(address);
      // Redirect to sequencer blocks table
      navigate(`/table/sequencer-blocks?address=${address}&range=${timeRange}`, { replace: true });
    } else {
      // Invalid sequencer address, redirect to dashboard
      navigate('/', { replace: true });
    }
  }, [address, sequencerList, setSelectedSequencer, navigate, timeRange]);

  return <div>Redirecting...</div>;
};