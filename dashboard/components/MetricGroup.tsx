import React from "react";

interface MetricGroupProps {
  title: string;
  children: React.ReactNode;
}

export const MetricGroup: React.FC<MetricGroupProps> = ({
  title,
  children,
}) => (
  <section className="mb-6">
    <h2 className="text-lg font-semibold text-gray-700 mb-2">{title}</h2>
    <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-5 xl:grid-cols-6 2xl:grid-cols-8 gap-4 md:gap-6">
      {children}
    </div>
  </section>
);
