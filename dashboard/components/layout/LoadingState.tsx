import React from 'react';
// brand color via CSS variable

interface LoadingStateProps {
  message?: string;
}

export const LoadingState: React.FC<LoadingStateProps> = ({
  message = 'Loading...',
}) => {
  return (
    <div className="p-4">
      <div className="flex items-center space-x-2">
        <div
          className="animate-spin rounded-full h-4 w-4 border-b-2"
          style={{ borderColor: 'var(--color-brand)' }}
        />
        <span>{message}</span>
      </div>
    </div>
  );
};
