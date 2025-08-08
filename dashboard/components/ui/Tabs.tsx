import React from 'react';
import { cn } from '../../utils/cn';

export const TabList: React.FC<React.PropsWithChildren<{ className?: string }>> = ({ className, children }) => (
  <div role="tablist" className={cn('flex items-center gap-2', className)}>
    {children}
  </div>
);

export interface TabProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  selected?: boolean;
}

export const Tab: React.FC<TabProps> = ({ selected = false, className, children, ...props }) => {
  const base = 'px-2 py-1 text-sm rounded-md border transition-colors';
  const appearance = selected
    ? 'bg-muted border-[var(--ring)] text-[var(--color-brand)] font-semibold'
    : 'bg-card border-border text-[var(--muted-fg)] hover:text-[var(--color-brand)] hover:bg-muted';

  return (
    <button role="tab" aria-selected={selected} className={cn(base, appearance, className)} {...props}>
      {children}
    </button>
  );
};
