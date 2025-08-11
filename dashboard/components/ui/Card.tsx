import React from 'react';
import { cn } from '../../utils/cn';

export type CardProps = React.PropsWithChildren<{
  className?: string;
}>;

export const Card: React.FC<CardProps> = ({ className, children }) => {
  return (
    <div
      className={cn(
        'bg-card text-card-fg border rounded-lg border-slate-300 dark:border-0 dark:bg-[rgba(30,41,59,0.85)]',
        'shadow-sm',
        className,
      )}
    >
      {children}
    </div>
  );
};

export const CardHeader: React.FC<CardProps> = ({ className, children }) => (
  <div className={cn('px-3 sm:px-4 md:px-6 pt-3 sm:pt-4 md:pt-6', className)}>
    {children}
  </div>
);

export const CardBody: React.FC<CardProps> = ({ className, children }) => (
  <div className={cn('px-3 sm:px-4 md:px-6 pb-3 sm:pb-4 md:pb-6', className)}>
    {children}
  </div>
);

export const CardTitle: React.FC<CardProps> = ({ className, children }) => (
  <h3 className={cn('text-lg font-semibold text-muted-fg', className)}>
    {children}
  </h3>
);
