import React from 'react';
import { cn } from '../../utils/cn';

export type InputProps = React.InputHTMLAttributes<HTMLInputElement>;

export const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ className, ...props }, ref) => {
    return (
      <input
        ref={ref}
        className={cn(
          'px-3 py-2 border border-gray-100 dark:border-border rounded-md bg-card text-card-fg',
          'focus:outline-none focus:ring-2 focus:ring-ring',
          className,
        )}
        {...props}
      />
    );
  },
);
Input.displayName = 'Input';
