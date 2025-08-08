/* eslint-disable react/prop-types */
import React from 'react';
import { cn } from '../../utils/cn';

export type SelectProps = React.SelectHTMLAttributes<HTMLSelectElement>;

export const Select = React.forwardRef<HTMLSelectElement, SelectProps>(
  ({ className, children, ...props }, ref) => {
    return (
      <select
        ref={ref}
        className={cn(
          'px-2 py-1 border border-gray-100 dark:border-border rounded-md bg-card text-card-fg',
          'focus:outline-none focus:ring-2 focus:ring-ring',
          className,
        )}
        {...props}
      >
        {children}
      </select>
    );
  },
);
Select.displayName = 'Select';
