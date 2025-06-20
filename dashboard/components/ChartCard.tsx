import React, { Suspense } from 'react';
import { LazyMount } from './LazyMount';

type ChartCardProps = React.PropsWithChildren<{
  title: string;
  onMore?: () => void;
  loading?: boolean;
}>;

export const ChartCard: React.FC<ChartCardProps> = ({
  title,
  children,
  onMore,
  loading,
}) => {
  return (
    <div className="bg-white dark:bg-gray-800 p-3 sm:p-4 md:p-6 rounded-lg border border-gray-200 dark:border-gray-700 relative">
      <div className="flex justify-between items-start mb-4">
        <h3 className="text-lg font-semibold text-gray-700 dark:text-gray-300">
          {title}
        </h3>
        {onMore && (
          <button
            onClick={onMore}
            className="text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300 text-2xl w-8 h-8 flex items-center justify-center rounded-md"
            aria-label="View table"
          >
            ⋮
          </button>
        )}
      </div>
      <div className="h-64 md:h-80 w-full relative">
        <LazyMount>
          <Suspense
            fallback={
              <div className="flex items-center justify-center h-full text-gray-500 dark:text-gray-400">
                Loading...
              </div>
            }
          >
            {children}
          </Suspense>
        </LazyMount>
        {loading && (
          <div className="absolute inset-0 flex items-center justify-center bg-white/60 dark:bg-gray-800/60">
            <span className="text-gray-500 dark:text-gray-400">Loading...</span>
          </div>
        )}
      </div>
    </div>
  );
};
