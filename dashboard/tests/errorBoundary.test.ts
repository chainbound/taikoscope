import { describe, it, expect, vi } from 'vitest';
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

  it('calls reportError when provided', () => {
    const report = vi.fn();
    const boundary = new ErrorBoundary({ reportError: report });
    const error = new Error('boom');
    const info = { componentStack: 'stack' } as React.ErrorInfo;
    boundary.componentDidCatch(error, info);
    expect(report).toHaveBeenCalledWith(error, info);
  });

  it('resetError clears error state', () => {
    const boundary = new ErrorBoundary({});
    boundary.setState({ hasError: true, error: new Error('oops'), info: undefined });
    (boundary as any).resetError();
    expect(boundary.state.hasError).toBe(false);
    expect(boundary.state.error).toBeUndefined();
  });
});
