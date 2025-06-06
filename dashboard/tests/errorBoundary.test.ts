import { describe, it, expect } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { ErrorBoundary } from '../components/ErrorBoundary';

describe('ErrorBoundary', () => {
  it('renders children', () => {
    const html = renderToStaticMarkup(
      React.createElement(
        ErrorBoundary,
        null,
        React.createElement('div', null, 'ok'),
      ),
    );
    expect(html).toContain('ok');
  });
});
