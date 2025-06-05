import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';

const navSpy = vi.fn();

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return {
    ...actual,
    useNavigate: () => navSpy,
    useSearchParams: () => [new URLSearchParams(), vi.fn()],
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
});
