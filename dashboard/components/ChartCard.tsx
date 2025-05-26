import React from "react";

interface ChartCardProps {
  title: string;
  children: React.ReactNode;
}

export const ChartCard: React.FC<ChartCardProps> = ({ title, children }) => {
  return (
    <div className="bg-gray-800 p-4 md:p-6 rounded-lg border border-gray-700">
      <h3 className="text-lg font-semibold text-gray-200 mb-4">{title}</h3>
      <div className="h-64 md:h-80 w-full">
        {" "}
        {/* Ensure chart has height */}
        {children}
      </div>
    </div>
  );
};
