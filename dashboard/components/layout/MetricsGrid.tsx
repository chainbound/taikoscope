import React from 'react';
import { MetricCard } from '../MetricCard';
import { MetricCardSkeleton } from '../MetricCardSkeleton';
import { MetricData } from '../../types';

interface MetricsGridProps {
  isLoading: boolean;
  groupedMetrics: Record<string, MetricData[]>;
  groupOrder: string[];
  skeletonGroupCounts: Record<string, number>;
  displayGroupName: (group: string) => string;
  onMetricAction: (title: string) => (() => void) | undefined;
}

export const MetricsGrid: React.FC<MetricsGridProps> = ({
  isLoading,
  groupedMetrics,
  groupOrder,
  skeletonGroupCounts,
  displayGroupName,
  onMetricAction,
}) => {
  const displayedGroupOrder = groupOrder;

  return (
    <>
      {(isLoading ? Object.keys(skeletonGroupCounts) : displayedGroupOrder).map(
        (group) =>
          isLoading ? (
            <React.Fragment key={group}>
              {group !== 'Other' && (
                <h2 className="mt-6 mb-2 text-lg font-semibold">
                  {displayGroupName(group)}
                </h2>
              )}
              <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-5 xl:grid-cols-6 2xl:grid-cols-8 gap-4 md:gap-6">
                {Array.from({ length: skeletonGroupCounts[group] }).map(
                  (_, idx) => (
                    <MetricCardSkeleton key={`${group}-s-${idx}`} />
                  ),
                )}
              </div>
            </React.Fragment>
          ) : groupedMetrics[group] && groupedMetrics[group].length > 0 ? (
            <React.Fragment key={group}>
              {group !== 'Other' && (
                <h2 className="mt-6 mb-2 text-lg font-semibold">
                  {displayGroupName(group)}
                </h2>
              )}
              <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-5 xl:grid-cols-6 2xl:grid-cols-8 gap-4 md:gap-6">
                {groupedMetrics[group].map((m, idx) => (
                  <MetricCard
                    key={`${group}-${idx}`}
                    title={m.title}
                    value={m.value}
                    onMore={
                      typeof m.title === 'string'
                        ? onMetricAction(m.title)
                        : undefined
                    }
                  />
                ))}
              </div>
            </React.Fragment>
          ) : null,
      )}
    </>
  );
};
