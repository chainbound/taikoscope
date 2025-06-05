import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { ErrorBoundary } from './components/ErrorBoundary';
import { ToastProvider } from './components/ToastProvider';
import { ErrorProvider } from './hooks/useErrorHandler';
import { BrowserRouter } from 'react-router-dom';
import './index.css';

const rootElement = document.getElementById('root');
if (!rootElement) {
  throw new Error('Could not find root element to mount to');
}

const root = ReactDOM.createRoot(rootElement);
root.render(
  <React.StrictMode>
    <ToastProvider>
      <ErrorBoundary>
        <ErrorProvider>
          <BrowserRouter>
            <App />
          </BrowserRouter>
        </ErrorProvider>
      </ErrorBoundary>
    </ToastProvider>
  </React.StrictMode>,
);
