import React, { lazy } from 'react';
import { ChartCard } from '../ChartCard';
import { TAIKO_PINK } from '../../theme';
import { TimeRange, TimeSeriesData, PieChartDataItem } from '../../types';
import type { BlockTransaction, BatchBlobCount } from '../../services/apiService';

const SequencerPieChart = lazy(() =>
    import('../SequencerPieChart').then((m) => ({
        default: m.SequencerPieChart,
    })),
);
const BlockTimeChart = lazy(() =>
    import('../BlockTimeChart').then((m) => ({
        default: m.BlockTimeChart,
    })),
);
const BatchProcessChart = lazy(() =>
    import('../BatchProcessChart').then((m) => ({
        default: m.BatchProcessChart,
    })),
);
const GasUsedChart = lazy(() =>
    import('../GasUsedChart').then((m) => ({
        default: m.GasUsedChart,
    })),
);
const BlockTxChart = lazy(() =>
    import('../BlockTxChart').then((m) => ({
        default: m.BlockTxChart,
    })),
);
const BlobsPerBatchChart = lazy(() =>
    import('../BlobsPerBatchChart').then((m) => ({
        default: m.BlobsPerBatchChart,
    })),
);

interface ChartsGridProps {
    isLoading: boolean;
    timeRange: TimeRange;
    selectedSequencer: string | null;
    chartsData: {
        sequencerDistribution: PieChartDataItem[];
        secondsToProveData: TimeSeriesData[];
        secondsToVerifyData: TimeSeriesData[];
        l2GasUsedData: TimeSeriesData[];
        blockTxData: BlockTransaction[];
        batchBlobCounts: BatchBlobCount[];
        l2BlockTimeData: TimeSeriesData[];
    };
    onOpenTable: (table: string, timeRange?: TimeRange) => void;
    onOpenSequencerDistributionTable: (timeRange: TimeRange, page: number) => void;
}

export const ChartsGrid: React.FC<ChartsGridProps> = ({
    isLoading,
    timeRange,
    selectedSequencer,
    chartsData,
    onOpenTable,
    onOpenSequencerDistributionTable,
}) => {
    return (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 md:gap-6 mt-6">
            {!selectedSequencer && (
                <ChartCard
                    title="Sequencer Distribution"
                    onMore={() => onOpenSequencerDistributionTable(timeRange, 0)}
                    loading={isLoading}
                >
                    <SequencerPieChart
                        key={timeRange}
                        data={chartsData.sequencerDistribution}
                    />
                </ChartCard>
            )}
            <ChartCard
                title="Prove Time"
                onMore={() => onOpenTable('prove-time', timeRange)}
                loading={isLoading}
            >
                <BatchProcessChart
                    key={timeRange}
                    data={chartsData.secondsToProveData}
                    lineColor={TAIKO_PINK}
                />
            </ChartCard>
            <ChartCard
                title="Verify Time"
                onMore={() => onOpenTable('verify-time', timeRange)}
                loading={isLoading}
            >
                <BatchProcessChart
                    key={timeRange}
                    data={chartsData.secondsToVerifyData}
                    lineColor="#5DA5DA"
                />
            </ChartCard>
            <ChartCard title="Gas Used Per Block" loading={isLoading}>
                <GasUsedChart
                    key={timeRange}
                    data={chartsData.l2GasUsedData}
                    lineColor="#E573B5"
                />
            </ChartCard>
            <ChartCard
                title="Tx Count Per Block"
                onMore={() => onOpenTable('block-tx', timeRange)}
                loading={isLoading}
            >
                <BlockTxChart
                    key={timeRange}
                    data={chartsData.blockTxData}
                    barColor="#4E79A7"
                />
            </ChartCard>
            <ChartCard
                title="Blobs per Batch"
                onMore={() => onOpenTable('blobs-per-batch', timeRange)}
                loading={isLoading}
            >
                <BlobsPerBatchChart
                    key={timeRange}
                    data={chartsData.batchBlobCounts}
                    barColor="#A0CBE8"
                />
            </ChartCard>
            <ChartCard
                title="L2 Block Times"
                onMore={() => onOpenTable('l2-block-times', timeRange)}
                loading={isLoading}
            >
                <BlockTimeChart
                    key={timeRange}
                    data={chartsData.l2BlockTimeData}
                    lineColor="#FAA43A"
                />
            </ChartCard>
        </div>
    );
};