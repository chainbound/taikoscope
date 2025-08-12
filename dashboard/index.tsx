import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { ErrorBoundary } from './components/ErrorBoundary';
import { ToastProvider } from './components/ToastProvider';
import { ErrorProvider } from './hooks/useErrorHandler';
import { BrowserRouter } from 'react-router-dom';
import './index.css';
import './styles/recharts.css';
import 'react-day-picker/dist/style.css';
import { isMainnet } from './utils';

const rootElement = document.getElementById('root');
if (!rootElement) {
  throw new Error('Could not find root element to mount to');
}

const root = ReactDOM.createRoot(rootElement);

// Inject analytics script only on mainnet
if (isMainnet) {
  const script = document.createElement('script');
  script.defer = true;
  script.src = 'https://umami.chainbound.io/script.js';
  script.setAttribute('data-website-id', '82893695-a8ae-42bd-870d-903850eab2b9');
  document.head.appendChild(script);
}

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
