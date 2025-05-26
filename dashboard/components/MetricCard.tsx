import React from "react";

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
  return (
    <div className="bg-gray-800 p-4 rounded-lg border border-gray-700 transition-shadow duration-200">
      {/* Removed {unit && `(${unit})`} from here */}
      <h3 className="text-sm font-medium text-gray-400 truncate">{title}</h3>
      <p className="mt-1 text-3xl font-semibold text-white">{value}</p>
      {description && (
        <p className="text-xs text-gray-400 mt-1">{description}</p>
      )}
    </div>
  );
};
