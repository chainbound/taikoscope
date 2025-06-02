import React, { Suspense } from 'react';

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
    <div className="bg-white p-4 md:p-6 rounded-lg border border-gray-200 relative">
      <div className="flex justify-between items-start mb-4">
        <h3 className="text-lg font-semibold text-gray-700">{title}</h3>
        {onMore && (
          <button
            onClick={onMore}
            className="text-gray-500 hover:text-gray-700 text-2xl w-8 h-8 flex items-center justify-center rounded-md"
            aria-label="View table"
          >
            ⋮
          </button>
        )}
      </div>
      <div className="h-64 md:h-80 w-full relative">
        <Suspense
          fallback={
            <div className="flex items-center justify-center h-full text-gray-500">
              Loading...
            </div>
          }
        >
          {children}
        </Suspense>
        {loading && (
          <div className="absolute inset-0 flex items-center justify-center bg-white/60">
            <span className="text-gray-500">Loading...</span>
          </div>
        )}
      </div>
    </div>
  );
};
