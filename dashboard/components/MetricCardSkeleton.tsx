import React from 'react';

export const MetricCardSkeleton: React.FC = () => (
  <div className="bg-card text-card-fg p-4 rounded-lg border border-slate-300 dark:border-0 dark:bg-[rgba(30,41,59,0.85)] animate-pulse">
    <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-1/2 mb-2" />
    <div className="h-6 bg-gray-200 dark:bg-gray-700 rounded w-3/4" />
  </div>
);
