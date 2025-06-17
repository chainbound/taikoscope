import { useState, useCallback, useMemo } from 'react';
import { TimeSeriesData, PieChartDataItem } from '../types';
import type { BlockTransaction, BatchBlobCount } from '../services/apiService';

export const useChartsData = () => {
  const [secondsToProveData, setSecondsToProveData] = useState<
    TimeSeriesData[]
  >([]);
  const [secondsToVerifyData, setSecondsToVerifyData] = useState<
    TimeSeriesData[]
  >([]);
  const [l2BlockTimeData, setL2BlockTimeData] = useState<TimeSeriesData[]>([]);
  const [l2GasUsedData, setL2GasUsedData] = useState<TimeSeriesData[]>([]);
  const [blockTxData, setBlockTxData] = useState<BlockTransaction[]>([]);
  const [batchBlobCounts, setBatchBlobCounts] = useState<BatchBlobCount[]>([]);
  const [sequencerDistribution, setSequencerDistribution] = useState<
    PieChartDataItem[]
  >([]);

  interface ChartsDataUpdate {
    proveTimes?: TimeSeriesData[];
    verifyTimes?: TimeSeriesData[];
    l2Times?: TimeSeriesData[];
    l2Gas?: TimeSeriesData[];
    txPerBlock?: BlockTransaction[];
    blobsPerBatch?: BatchBlobCount[];
    sequencerDist?: PieChartDataItem[];
  }

  const updateChartsData = useCallback(
    (data: ChartsDataUpdate) => {
      if (data.proveTimes) setSecondsToProveData([...data.proveTimes]);
      if (data.verifyTimes) setSecondsToVerifyData([...data.verifyTimes]);
      if (data.l2Times) setL2BlockTimeData([...data.l2Times]);
      if (data.l2Gas) setL2GasUsedData([...data.l2Gas]);
      if (data.txPerBlock) setBlockTxData([...data.txPerBlock]);
      if (data.blobsPerBatch) setBatchBlobCounts([...data.blobsPerBatch]);
      if (data.sequencerDist)
        setSequencerDistribution([...data.sequencerDist]);
    },
    [],
  );

  return useMemo(
    () => ({
      secondsToProveData,
      secondsToVerifyData,
      l2BlockTimeData,
      l2GasUsedData,
      blockTxData,
      batchBlobCounts,
      sequencerDistribution,
      updateChartsData,
    }),
    [
      secondsToProveData,
      secondsToVerifyData,
      l2BlockTimeData,
      l2GasUsedData,
      blockTxData,
      batchBlobCounts,
      sequencerDistribution,
      updateChartsData,
    ],
  );
};
