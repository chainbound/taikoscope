import { useState, useEffect, useCallback } from 'react';
import { fetchL1HeadNumber, fetchL2HeadNumber } from '../services/apiService';
import { MetricData } from '../types';

export const useBlockData = () => {
    const [l2HeadBlock, setL2HeadBlock] = useState<string>('0');
    const [l1HeadBlock, setL1HeadBlock] = useState<string>('0');

    const updateBlockHeads = useCallback(async () => {
        const [l1, l2] = await Promise.all([
            fetchL1HeadNumber(),
            fetchL2HeadNumber(),
        ]);

        if (l1.data !== null) {
            const value = l1.data.toLocaleString();
            setL1HeadBlock(value);
        }

        if (l2.data !== null) {
            const value = l2.data.toLocaleString();
            setL2HeadBlock(value);
        }
    }, []);

    const updateMetricsWithBlockHeads = useCallback(
        (metrics: MetricData[]): MetricData[] => {
            return metrics.map((metric) => {
                if (metric.title === 'L1 Block') {
                    return { ...metric, value: l1HeadBlock };
                }
                if (metric.title === 'L2 Block') {
                    return { ...metric, value: l2HeadBlock };
                }
                return metric;
            });
        },
        [l1HeadBlock, l2HeadBlock],
    );

    useEffect(() => {
        updateBlockHeads();
        const pollId = setInterval(updateBlockHeads, 60000);
        return () => clearInterval(pollId);
    }, [updateBlockHeads]);

    return {
        l2HeadBlock,
        l1HeadBlock,
        updateBlockHeads,
        updateMetricsWithBlockHeads,
    };
};