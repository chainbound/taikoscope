import React from 'react';

export const MetricCardSkeleton: React.FC = () => (
  <div className="bg-white p-4 rounded-lg border border-gray-200 animate-pulse">
    <div className="h-4 bg-gray-200 rounded w-1/2 mb-2" />
    <div className="h-6 bg-gray-200 rounded w-3/4" />
  </div>
);
