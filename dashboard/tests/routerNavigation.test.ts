import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';

const navSpy = vi.fn();
let currentSearchParams = new URLSearchParams();

vi.mock('react-router-dom', async () => {
  const actual =
    await vi.importActual<typeof import('react-router-dom')>(
      'react-router-dom',
    );
  return {
    ...actual,
    useNavigate: () => navSpy,
    useSearchParams: () => [currentSearchParams, vi.fn()],
  };
});

import { useRouterNavigation } from '../hooks/useRouterNavigation';

const mockLocation = {
  origin: 'https://example.com',
  pathname: '/dashboard',
  href: 'https://example.com/dashboard',
};

describe('useRouterNavigation', () => {
  beforeEach(() => {
    vi.stubGlobal('window', { location: mockLocation, history: { length: 1 } });
    navSpy.mockClear();
    currentSearchParams = new URLSearchParams();
  });

  it('sanitizes table navigation', () => {
    function Test() {
      const { navigateToTable } = useRouterNavigation();
      navigateToTable('blocks', { page: -1 });
      return null;
    }

    renderToStaticMarkup(React.createElement(Test));
    expect(navSpy).toHaveBeenCalledWith('/table/blocks', { replace: false });
  });

  it('cleans params in updateSearchParams', () => {
    function Test() {
      const { updateSearchParams } = useRouterNavigation();
      updateSearchParams({ view: 'table', page: '-1' });
      return null;
    }

    renderToStaticMarkup(React.createElement(Test));
    expect(navSpy).toHaveBeenCalledWith('/?view=table', { replace: true });
  });

  it('preserves view param when navigating to a table', () => {
    currentSearchParams = new URLSearchParams('view=health');

    function Test() {
      const { navigateToTable } = useRouterNavigation();
      navigateToTable('verify-times', undefined, '24h');
      return null;
    }

    renderToStaticMarkup(React.createElement(Test));
    expect(navSpy).toHaveBeenCalledWith(
      '/table/verify-times?range=24h&view=health',
      { replace: false },
    );
  });
});
