import { useState, useEffect, useMemo, useRef } from 'react';
import { useSearchParams } from 'react-router-dom';

interface UseSequencerHandlerProps {
  chartsData: {
    sequencerDistribution: Array<{ name: string }>;
  };
  blockData: {
    l1HeadBlock: string;
    l2HeadBlock: string;
    updateMetricsWithBlockHeads: (metrics: any[]) => any[];
  };
  metricsData: {
    metrics: any[];
    setMetrics: (metrics: any[]) => void;
  };
}

export const useSequencerHandler = ({
  chartsData,
  blockData,
  metricsData,
}: UseSequencerHandlerProps) => {
  const [searchParams] = useSearchParams();
  const [selectedSequencer, setSelectedSequencer] = useState<string | null>(
    searchParams.get('sequencer'),
  );

  const sequencerList = useMemo(
    () => chartsData.sequencerDistribution.map((s) => s.name),
    [chartsData.sequencerDistribution],
  );

  // Sync with URL params - extract specific value to avoid object dependency
  const sequencerParam = searchParams.get('sequencer');
  useEffect(() => {
    setSelectedSequencer(sequencerParam ?? null);
  }, [sequencerParam]);

  // Update metrics with current block heads whenever they change
  const lastUpdateRef = useRef<string>('');

  useEffect(() => {
    const currentKey = `${blockData.l1HeadBlock}-${blockData.l2HeadBlock}`;

    if (
      metricsData.metrics.length > 0 &&
      lastUpdateRef.current !== currentKey
    ) {
      const updatedMetrics = blockData.updateMetricsWithBlockHeads(
        metricsData.metrics,
      );
      metricsData.setMetrics(updatedMetrics);
      lastUpdateRef.current = currentKey;
    }
  }, [blockData.l1HeadBlock, blockData.l2HeadBlock]);

  return {
    selectedSequencer,
    setSelectedSequencer,
    sequencerList,
  };
};
