import { useState } from 'react';
import { TimeSeriesData, PieChartDataItem } from '../types';
import type { BlockTransaction, BatchBlobCount } from '../services/apiService';

export const useChartsData = () => {
    const [secondsToProveData, setSecondsToProveData] = useState<TimeSeriesData[]>([]);
    const [secondsToVerifyData, setSecondsToVerifyData] = useState<TimeSeriesData[]>([]);
    const [l2BlockTimeData, setL2BlockTimeData] = useState<TimeSeriesData[]>([]);
    const [l2GasUsedData, setL2GasUsedData] = useState<TimeSeriesData[]>([]);
    const [l1BlockTimeData, setL1BlockTimeData] = useState<TimeSeriesData[]>([]);
    const [blockTxData, setBlockTxData] = useState<BlockTransaction[]>([]);
    const [batchBlobCounts, setBatchBlobCounts] = useState<BatchBlobCount[]>([]);
    const [sequencerDistribution, setSequencerDistribution] = useState<PieChartDataItem[]>([]);

    const updateChartsData = (data: {
        proveTimes: TimeSeriesData[];
        verifyTimes: TimeSeriesData[];
        l2Times: TimeSeriesData[];
        l2Gas: TimeSeriesData[];
        l1Times: TimeSeriesData[];
        txPerBlock: BlockTransaction[];
        blobsPerBatch: BatchBlobCount[];
        sequencerDist: PieChartDataItem[];
    }) => {
        setSecondsToProveData(data.proveTimes);
        setSecondsToVerifyData(data.verifyTimes);
        setL2BlockTimeData(data.l2Times);
        setL2GasUsedData(data.l2Gas);
        setL1BlockTimeData(data.l1Times);
        setBlockTxData(data.txPerBlock);
        setBatchBlobCounts(data.blobsPerBatch);
        setSequencerDistribution(data.sequencerDist);
    };

    return {
        secondsToProveData,
        secondsToVerifyData,
        l2BlockTimeData,
        l2GasUsedData,
        l1BlockTimeData,
        blockTxData,
        batchBlobCounts,
        sequencerDistribution,
        updateChartsData,
    };
};