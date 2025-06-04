import React from 'react';

interface ErrorBoundaryProps {
  fallback?: React.ReactNode;
  /**
   * Optional function invoked when an error is caught. Can be used to
   * send the error to an external reporting service.
   */
  reportError?: (error: Error, info: React.ErrorInfo) => void;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error?: Error;
  info?: React.ErrorInfo;
}

export class ErrorBoundary extends React.Component<
  React.PropsWithChildren<ErrorBoundaryProps>,
  ErrorBoundaryState
> {
  constructor(props: React.PropsWithChildren<ErrorBoundaryProps>) {
    super(props);
    this.state = { hasError: false, error: undefined, info: undefined };
  }

  static getDerivedStateFromError(): ErrorBoundaryState {
    return { hasError: true };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    // eslint-disable-next-line no-console
    console.error('Error boundary caught an error:', error, info);
    this.setState({ error, info });
    if (this.props.reportError) {
      try {
        this.props.reportError(error, info);
      } catch (reportError) {
        // eslint-disable-next-line no-console
        console.error('Failed to report error', reportError);
      }
    }
  }

  render() {
    if (this.state.hasError) {
      return (
        this.props.fallback || (
          <div className="p-4 bg-red-50 border border-red-200 rounded text-red-700 space-y-2">
            <div>Oops! Something went wrong. Please reload the page.</div>
            <button
              onClick={() => window.location.reload()}
              className="text-sm bg-red-600 text-white px-2 py-1 rounded hover:bg-red-700"
            >
              Reload
            </button>
          </div>
        )
      );
    }

    return this.props.children;
  }
}
