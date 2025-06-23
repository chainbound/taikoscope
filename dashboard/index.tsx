import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { ErrorBoundary } from './components/ErrorBoundary';
import { ToastProvider } from './components/ToastProvider';
import { ErrorProvider } from './hooks/useErrorHandler';
import { BrowserRouter } from 'react-router-dom';
import './index.css';
import 'react-day-picker/dist/style.css';

const rootElement = document.getElementById('root');
if (!rootElement) {
  throw new Error('Could not find root element to mount to');
}

const root = ReactDOM.createRoot(rootElement);

const app = (
  <ToastProvider>
    <ErrorProvider>
      <ErrorBoundary>
        <BrowserRouter>
          <App />
        </BrowserRouter>
      </ErrorBoundary>
    </ErrorProvider>
  </ToastProvider>
);

root.render(
  process.env.NODE_ENV === 'development' ? (
    <React.StrictMode>{app}</React.StrictMode>
  ) : (
    app
  ),
);
