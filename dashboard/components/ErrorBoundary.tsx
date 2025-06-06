import React from 'react';
import {
  ErrorBoundary as ReactErrorBoundary,
  FallbackProps,
} from 'react-error-boundary';

interface ErrorBoundaryProps {
  fallback?: React.ReactNode;
  /**
   * Optional function invoked when an error is caught. Can be used to
   * send the error to an external reporting service.
   */
  reportError?: (error: Error, info: React.ErrorInfo) => void;
}

export const ErrorBoundary: React.FC<
  React.PropsWithChildren<ErrorBoundaryProps>
> = ({ fallback, reportError, children }) => {
  const handleError = (error: Error, info: React.ErrorInfo) => {
    console.error('Error boundary caught an error:', error, info);
    if (reportError) {
      try {
        reportError(error, info);
      } catch (reportError) {
        console.error('Failed to report error', reportError);
      }
    }
  };

  const FallbackComponent = ({
    resetErrorBoundary,
  }: FallbackProps): React.ReactElement => {
    if (fallback) return <>{fallback}</>;
    return (
      <div className="p-4 bg-red-50 border border-red-200 rounded text-red-700 space-y-2">
        <div>Oops! Something went wrong.</div>
        <button
          onClick={resetErrorBoundary}
          className="text-sm bg-red-600 text-white px-2 py-1 rounded hover:bg-red-700"
        >
          Retry
        </button>
      </div>
    );
  };

  return (
    <ReactErrorBoundary
      FallbackComponent={FallbackComponent}
      onError={handleError}
    >
      {children}
    </ReactErrorBoundary>
  );
};
