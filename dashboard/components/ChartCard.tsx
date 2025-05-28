import React from 'react';

interface ChartCardProps {
  title: string;
  children: React.ReactNode;
  onMore?: () => void;
}

export const ChartCard: React.FC<ChartCardProps> = ({
  title,
  children,
  onMore,
}) => {
  return (
    <div className="bg-white p-4 md:p-6 rounded-lg border border-gray-200 relative">
      <div className="flex justify-between items-start mb-4">
        <h3 className="text-lg font-semibold text-gray-700">{title}</h3>
        {onMore && (
          <button
            onClick={onMore}
            className="text-gray-500 hover:text-gray-700"
            aria-label="View table"
          >
            ⋮
          </button>
        )}
      </div>
      <div className="h-64 md:h-80 w-full">{children}</div>
    </div>
  );
};
