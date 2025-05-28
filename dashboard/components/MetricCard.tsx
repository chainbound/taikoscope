import React from 'react';

interface MetricCardProps {
  title: React.ReactNode;
  value: string;
  unit?: string; // Unit is passed but not displayed in the title directly as (unit)
  description?: React.ReactNode;
  className?: string;
  onMore?: () => void;
}

export const MetricCard: React.FC<MetricCardProps> = ({
  title,
  value,
  description,
  className,
  onMore,
}) => {
  // Check if value looks like an Ethereum address (0x followed by 40 hex characters)
  const isAddress = /^0x[a-fA-F0-9]{40}$/.test(value);

  return (
    <div
      className={`bg-white p-4 rounded-lg border border-gray-200 transition-shadow duration-200 ${isAddress ? 'min-w-0 w-full sm:col-span-2' : ''} ${className ?? ''}`.trim()}
    >
      <div className="flex justify-between items-start">
        <h3 className="text-sm font-medium text-gray-500 truncate">{title}</h3>
        {onMore && (
          <button
            onClick={onMore}
            className="text-gray-500 hover:text-gray-700 text-2xl"
            aria-label="View table"
          >
            ⋮
          </button>
        )}
      </div>
      <p
        className={`mt-1 font-semibold text-gray-900 ${isAddress ? 'text-lg whitespace-nowrap' : 'text-3xl break-all'}`}
      >
        {value}
      </p>
      {description && (
        <p className="text-xs text-gray-400 mt-1">{description}</p>
      )}
    </div>
  );
};
