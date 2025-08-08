import React, { Suspense } from 'react';
import { LazyMount } from './LazyMount';
import { Card, CardBody, CardHeader, CardTitle } from './ui/Card';

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
    <Card className="relative">
      <CardHeader className="pb-0">
        <div className="flex justify-between items-start mb-4">
          <CardTitle>{title}</CardTitle>
          {onMore && (
            <button
              onClick={onMore}
              className="text-muted-fg hover:text-fg text-2xl w-8 h-8 flex items-center justify-center rounded-md"
              aria-label="View table"
            >
              ⋮
            </button>
          )}
        </div>
      </CardHeader>
      <CardBody className="pt-0">
        <div className="h-64 md:h-80 w-full relative">
          <LazyMount>
            <Suspense
              fallback={
                <div className="flex items-center justify-center h-full text-muted-fg">
                  Loading...
                </div>
              }
            >
              {children}
            </Suspense>
          </LazyMount>
          {loading && (
            <div className="absolute inset-0 flex items-center justify-center bg-white/60 dark:bg-gray-800/60">
              <span className="text-muted-fg">Loading...</span>
            </div>
          )}
        </div>
      </CardBody>
    </Card>
  );
};
