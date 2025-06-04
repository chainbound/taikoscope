import { useState, useEffect, useMemo } from 'react';
import { useSearchParams } from './useSearchParams';

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
    const searchParams = useSearchParams();
    const [selectedSequencer, setSelectedSequencer] = useState<string | null>(
        searchParams.get('sequencer'),
    );

    const sequencerList = useMemo(
        () => chartsData.sequencerDistribution.map((s) => s.name),
        [chartsData.sequencerDistribution],
    );

    // Sync with URL params
    useEffect(() => {
        const seq = searchParams.get('sequencer');
        setSelectedSequencer(seq ?? null);
    }, [searchParams]);

    // Update metrics with current block heads whenever they change
    useEffect(() => {
        if (metricsData.metrics.length > 0) {
            const updatedMetrics = blockData.updateMetricsWithBlockHeads(metricsData.metrics);
            metricsData.setMetrics(updatedMetrics);
        }
    }, [blockData.l1HeadBlock, blockData.l2HeadBlock, blockData.updateMetricsWithBlockHeads]);

    return {
        selectedSequencer,
        setSelectedSequencer,
        sequencerList,
    };
};