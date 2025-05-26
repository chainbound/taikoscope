import React from "react";

interface MetricSectionProps {
  title: React.ReactNode;
  children: React.ReactNode;
}

export const MetricSection: React.FC<MetricSectionProps> = ({
  title,
  children,
}) => (
  <section className="mt-6">
    <h2 className="text-lg font-semibold text-gray-700 mb-2">{title}</h2>
    <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-6 2xl:grid-cols-8 gap-4 md:gap-6">
      {children}
    </div>
  </section>
);
