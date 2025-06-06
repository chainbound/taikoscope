import React, { lazy } from 'react';
import { ChartCard } from '../ChartCard';
import { TAIKO_PINK } from '../../theme';
import { TimeRange, TimeSeriesData, PieChartDataItem } from '../../types';
import type {
  BlockTransaction,
  BatchBlobCount,
} from '../../services/apiService';

const SequencerPieChart = lazy(() =>
  import('../SequencerPieChart').then((m) => ({
    default: m.SequencerPieChart,
  })),
);
const BlockTimeDistributionChart = lazy(() =>
  import('../BlockTimeDistributionChart').then((m) => ({
    default: m.BlockTimeDistributionChart,
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
  isTimeRangeChanging?: boolean;
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
  onOpenSequencerDistributionTable: (
    timeRange: TimeRange,
    page: number,
  ) => void;
}

export const ChartsGrid: React.FC<ChartsGridProps> = ({
  isLoading,
  isTimeRangeChanging,
  timeRange,
  selectedSequencer,
  chartsData,
  onOpenTable,
  onOpenSequencerDistributionTable,
}) => {
  const networkPerformanceCharts = (
    <>
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
    </>
  );

  const networkHealthCharts = (
    <>
      <ChartCard
        title="Gas Used Per Block"
        onMore={() => onOpenTable('l2-gas-used', timeRange)}
        loading={isLoading}
      >
        <GasUsedChart
          key={timeRange}
          data={chartsData.l2GasUsedData}
          lineColor="#E573B5"
        />
      </ChartCard>
      <ChartCard
        title="Tx Count Per L2 Block"
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
        title="L2 Block Time Distribution"
        onMore={() => onOpenTable('l2-block-times', timeRange)}
        loading={isLoading}
      >
        <BlockTimeDistributionChart
          key={timeRange}
          data={chartsData.l2BlockTimeData}
          barColor="#FAA43A"
        />
      </ChartCard>
    </>
  );

  const sequencerCharts = !selectedSequencer ? (
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
  ) : null;

  return (
    <div className="mt-6">
      {isTimeRangeChanging && (
        <div className="mb-4 p-3 bg-blue-50 dark:bg-blue-900/20 rounded-lg border border-blue-200 dark:border-blue-800">
          <div className="flex items-center space-x-2">
            <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600 dark:border-blue-400"></div>
            <span className="text-sm text-blue-800 dark:text-blue-200">
              Updating data for {timeRange} time range...
            </span>
          </div>
        </div>
      )}

      {selectedSequencer && (
        <h2 className="mb-2 text-lg font-semibold">Sequencer Performance</h2>
      )}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 md:gap-6">
        {!selectedSequencer && sequencerCharts}
        {networkPerformanceCharts}
      </div>

      {selectedSequencer && (
        <h2 className="mt-6 mb-2 text-lg font-semibold">Sequencer Health</h2>
      )}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 md:gap-6">
        {networkHealthCharts}
      </div>
    </div>
  );
};
