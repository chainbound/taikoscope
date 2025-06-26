import { useState, useEffect, useCallback, useRef } from 'react';
import { fetchDashboardData } from '../services/apiService';
import { MetricData, BlockDataState } from '../types';
import { TAIKOSCAN_BASE } from '../utils';

export const useBlockData = (): BlockDataState => {
  const [l2HeadBlock, setL2HeadBlock] = useState<string>('0');
  const [l1HeadBlock, setL1HeadBlock] = useState<string>('0');
  const [candidates, setCandidates] = useState<string[]>([]);
  const pollId = useRef<ReturnType<typeof setInterval> | null>(null);

  const updateBlockHeads = useCallback(async () => {
    try {
      const res = await fetchDashboardData('1h');
      if (res.data?.l1_head_block != null) {
        setL1HeadBlock(res.data.l1_head_block.toLocaleString());
      }
      if (res.data?.l2_head_block != null) {
        setL2HeadBlock(res.data.l2_head_block.toLocaleString());
      }
      if (res.data?.preconf_data?.candidates) {
        setCandidates(res.data.preconf_data.candidates);
      }
    } catch (error) {
      console.error('Failed to update block heads:', error);
    }
  }, []);

  const updateMetricsWithBlockHeads = useCallback(
    (metrics: MetricData[]): MetricData[] => {
      return metrics.map((metric) => {
        if (metric.title === 'L1 Block') {
          const num = Number(l1HeadBlock.replace(/,/g, ''));
          return {
            ...metric,
            value: l1HeadBlock,
            link: Number.isFinite(num)
              ? `${TAIKOSCAN_BASE}/block/${num}`
              : metric.link,
          };
        }
        if (metric.title === 'L2 Block') {
          const num = Number(l2HeadBlock.replace(/,/g, ''));
          return {
            ...metric,
            value: l2HeadBlock,
            link: Number.isFinite(num)
              ? `${TAIKOSCAN_BASE}/block/${num}`
              : metric.link,
          };
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
    candidates,
    updateBlockHeads,
    updateMetricsWithBlockHeads,
  };
};
