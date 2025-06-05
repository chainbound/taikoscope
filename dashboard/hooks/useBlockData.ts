import { useState, useEffect, useCallback, useRef } from 'react';
import { fetchL1HeadNumber, fetchL2HeadNumber } from '../services/apiService';
import { MetricData } from '../types';

export const useBlockData = () => {
    const [l2HeadBlock, setL2HeadBlock] = useState<string>('0');
    const [l1HeadBlock, setL1HeadBlock] = useState<string>('0');
    const pollId = useRef<ReturnType<typeof setInterval> | null>(null);

    const updateBlockHeads = useCallback(async () => {
        try {
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
        } catch (error) {
            console.error('Failed to update block heads:', error);
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
        const startPolling = () => {
            pollId.current = setInterval(() => {
                if (document.visibilityState === 'visible') {
                    void updateBlockHeads();
                }
            }, 60000);
        };

        if (document.visibilityState === 'visible') {
            void updateBlockHeads();
            startPolling();
        }

        const onVisibilityChange = () => {
            if (document.visibilityState === 'visible') {
                void updateBlockHeads();
                if (!pollId.current) startPolling();
            } else if (pollId.current) {
                clearInterval(pollId.current);
                pollId.current = null;
            }
        };

        document.addEventListener('visibilitychange', onVisibilityChange);

        return () => {
            if (pollId.current) clearInterval(pollId.current);
            document.removeEventListener('visibilitychange', onVisibilityChange);
        };
    }, [updateBlockHeads]);

    return {
        l2HeadBlock,
        l1HeadBlock,
        updateBlockHeads,
        updateMetricsWithBlockHeads,
    };
};