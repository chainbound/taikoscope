import React from 'react';
import { cn } from '../../utils/cn';

export type ButtonVariant = 'primary' | 'ghost' | 'outline';
export type ButtonSize = 'sm' | 'md';

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
}

const base = 'inline-flex items-center justify-center rounded-md transition-colors focus:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50 disabled:pointer-events-none';
const sizes: Record<ButtonSize, string> = {
  sm: 'text-sm px-2 py-1',
  md: 'text-sm px-3 py-1.5',
};
const variants: Record<ButtonVariant, string> = {
  primary: 'bg-brand text-white hover:opacity-90',
  ghost: 'bg-transparent text-brand hover:bg-muted',
  outline: 'border border-border bg-transparent text-brand hover:bg-muted',
};

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant = 'primary', size = 'md', ...props }, ref) => {
    return (
      <button
        ref={ref}
        className={cn(base, sizes[size], variants[variant], className)}
        {...props}
      />
    );
  },
);
Button.displayName = 'Button';
