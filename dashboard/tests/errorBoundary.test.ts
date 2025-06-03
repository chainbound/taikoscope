import { describe, it, expect } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { ErrorBoundary } from '../components/ErrorBoundary';

describe('ErrorBoundary', () => {
  it('getDerivedStateFromError sets flag', () => {
    const state = ErrorBoundary.getDerivedStateFromError();
    expect(state).toEqual({ hasError: true });
  });

  it('renders fallback when hasError is true', () => {
    class TestBoundary extends ErrorBoundary {
      state = { hasError: true };
    }
    const html = renderToStaticMarkup(
      React.createElement(
        TestBoundary,
        { fallback: React.createElement('div', null, 'oops') },
        null,
      ),
    );
    expect(html).toContain('oops');
  });
});
