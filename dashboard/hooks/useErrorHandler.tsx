import React, { createContext, useContext, useState } from 'react';

interface ErrorContextValue {
  errorMessage: string;
  setErrorMessage: (msg: string) => void;
  clearError: () => void;
}

const ErrorContext = createContext<ErrorContextValue | undefined>(undefined);

export const ErrorProvider: React.FC<React.PropsWithChildren> = ({
  children,
}) => {
  const [errorMessage, setErrorMessage] = useState('');
  const clearError = () => setErrorMessage('');

  return (
    <ErrorContext.Provider
      value={{ errorMessage, setErrorMessage, clearError }}
    >
      {children}
    </ErrorContext.Provider>
  );
};

export const useErrorHandler = (): ErrorContextValue => {
  const ctx = useContext(ErrorContext);
  if (!ctx) {
    throw new Error('useErrorHandler must be used within ErrorProvider');
  }
  return ctx;
};
