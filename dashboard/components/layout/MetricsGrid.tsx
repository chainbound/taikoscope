import React from 'react';
import { MetricCard } from '../MetricCard';
import { MetricCardSkeleton } from '../MetricCardSkeleton';
import { MetricData, TimeRange } from '../../types';
import { formatTimeRangeDisplay } from '../../utils/timeRange';
import { parseEthValue, formatUsd } from '../../utils';
import { useEthPrice } from '../../services/priceService';

interface MetricsGridProps {
  isLoading: boolean;
  groupedMetrics: Record<string, MetricData[]>;
  groupOrder: string[];
  skeletonGroupCounts: Record<string, number>;
  displayGroupName: (group: string) => string;
  onMetricAction: (title: string) => (() => void) | undefined;
  economicsView?: boolean;
  groupedCharts?: Record<string, React.ReactNode[]>;
  isTimeRangeChanging?: boolean;
  timeRange?: TimeRange;
}

export const MetricsGrid: React.FC<MetricsGridProps> = ({
  isLoading,
  groupedMetrics,
  groupOrder,
  skeletonGroupCounts,
  displayGroupName,
  onMetricAction,
  economicsView,
  groupedCharts,
  isTimeRangeChanging,
  timeRange,
}) => {
  const displayedGroupOrder = groupOrder;
  const { data: ethPrice = 0 } = useEthPrice();
  const regularGrid =
    'grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-3 xl:grid-cols-3 2xl:grid-cols-3 gap-4 md:gap-6';
  const economicsGrid =
    'grid grid-cols-1 sm:grid-cols-2 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-3 2xl:grid-cols-3 gap-4 md:gap-6';
  const chartsGrid = 'grid grid-cols-1 lg:grid-cols-2 gap-4 md:gap-6 mt-4';

  return (
    <>
      {isTimeRangeChanging && timeRange && (
        <div className="mb-4 p-3 bg-blue-50 dark:bg-blue-900/20 rounded-lg border border-blue-200 dark:border-blue-800">
          <div className="flex items-center space-x-2">
            <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600 dark:border-blue-400"></div>
            <span className="text-sm text-blue-800 dark:text-blue-200">
              Updating data for {formatTimeRangeDisplay(timeRange)} time range...
            </span>
          </div>
        </div>
      )}
      {(isLoading ? Object.keys(skeletonGroupCounts) : displayedGroupOrder).map(
        (group) =>
          isLoading ? (
            <React.Fragment key={group}>
              {group !== 'Other' && (
                <h2 className={`${group === 'Network Economics' ? 'mt-2' : 'mt-6'} mb-2 text-lg font-semibold`}>
                  {displayGroupName(group)}
                </h2>
              )}
              <div className={economicsView ? economicsGrid : regularGrid}>
                {Array.from({ length: skeletonGroupCounts[group] }).map(
                  (_, idx) => (
                    <MetricCardSkeleton key={`${group}-s-${idx}`} />
                  ),
                )}
              </div>
              {groupedCharts?.[group] && groupedCharts[group].length > 0 && (
                <div className={chartsGrid}>{groupedCharts[group].map((c, i) => (
                  <React.Fragment key={`${group}-c-${i}`}>{c}</React.Fragment>
                ))}</div>
              )}
            </React.Fragment>
          ) : groupedMetrics[group] && groupedMetrics[group].length > 0 ? (
            <React.Fragment key={group}>
              {group !== 'Other' && (
                <h2 className={`${group === 'Network Economics' ? 'mt-2' : 'mt-6'} mb-2 text-lg font-semibold`}>
                  {displayGroupName(group)}
                </h2>
              )}
              <div className={economicsView ? economicsGrid : regularGrid}>
                {groupedMetrics[group].map((m, idx) => {
                  let valueTooltip = undefined;
                  if (economicsView && /ETH/i.test(m.value)) {
                    valueTooltip = `$${formatUsd(parseEthValue(m.value) * ethPrice)}`;
                  }
                  return (
                    <MetricCard
                      key={`${group}-${idx}`}
                      title={m.title}
                      value={m.value}
                      link={m.link}
                      onMore={
                        typeof m.title === 'string'
                          ? onMetricAction(m.title)
                          : undefined
                      }
                      tooltip={valueTooltip}
                      titleTooltip={m.tooltip}
                    />
                  );
                })}
              </div>
              {groupedCharts?.[group] && groupedCharts[group].length > 0 && (
                <div className={chartsGrid}>{groupedCharts[group].map((c, i) => (
                  <React.Fragment key={`${group}-c-${i}`}>{c}</React.Fragment>
                ))}</div>
              )}
            </React.Fragment>
          ) : null,
      )}
    </>
  );
};
