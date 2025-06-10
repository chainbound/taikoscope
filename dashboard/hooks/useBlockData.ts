import { useCallback } from 'react';
import useSWR from 'swr';
import { fetchL1HeadNumber, fetchL2HeadNumber } from '../services/apiService';
import { MetricData } from '../types';

export const useBlockData = () => {
  const {
    data: l1,
    mutate: mutateL1,
  } = useSWR('l1-head-number', fetchL1HeadNumber, {
    refreshInterval: 60000,
    revalidateOnFocus: false,
    refreshWhenHidden: false,
  });

  const {
    data: l2,
    mutate: mutateL2,
  } = useSWR('l2-head-number', fetchL2HeadNumber, {
    refreshInterval: 60000,
    revalidateOnFocus: false,
    refreshWhenHidden: false,
  });

  const l1HeadBlock = l1?.data != null ? l1.data.toLocaleString() : '0';
  const l2HeadBlock = l2?.data != null ? l2.data.toLocaleString() : '0';

  const updateBlockHeads = useCallback(async () => {
    await Promise.all([mutateL1(), mutateL2()]);
  }, [mutateL1, mutateL2]);

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


  return {
    l2HeadBlock,
    l1HeadBlock,
    updateBlockHeads,
    updateMetricsWithBlockHeads,
  };
};
