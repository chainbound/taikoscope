import React from 'react';

interface MetricCardProps {
  title: React.ReactNode;
  value: string;
  unit?: string; // Unit is passed but not displayed in the title directly as (unit)
  description?: React.ReactNode;
}

export const MetricCard: React.FC<MetricCardProps> = ({
  title,
  value,
  description,
}) => {
  // Check if value looks like an Ethereum address (0x followed by 40 hex characters)
  const isAddress = /^0x[a-fA-F0-9]{40}$/.test(value);

  return (
    <div className={`bg-white p-4 rounded-lg border border-gray-200 transition-shadow duration-200 ${isAddress ? 'min-w-0 w-full' : ''}`}>
      {/* Removed {unit && `(${unit})`} from here */}
      <h3 className="text-sm font-medium text-gray-500 truncate">{title}</h3>
      <p className={`mt-1 font-semibold text-gray-900 ${isAddress ? 'text-sm whitespace-nowrap overflow-x-auto' : 'text-3xl break-all'}`}>
        {value}
      </p>
      {description && (
        <p className="text-xs text-gray-400 mt-1">{description}</p>
      )}
    </div>
  );
};
