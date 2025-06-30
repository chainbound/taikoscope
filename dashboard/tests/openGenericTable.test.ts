import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';

const navSpy = vi.fn();
const mockLocation = {
  origin: 'https://example.com',
  pathname: '/',
  href: 'https://example.com/',
};

vi.mock('react-router-dom', async () => {
  const actual =
    await vi.importActual<typeof import('react-router-dom')>(
      'react-router-dom',
    );
  return {
    ...actual,
    useLocation: () => ({ pathname: '/' }),
    useNavigate: () => navSpy,
    useSearchParams: () => [new URLSearchParams(), vi.fn()],
  };
});

vi.mock('../hooks/useRouterNavigation', () => ({
  useRouterNavigation: () => ({ navigateToTable: navSpy }),
}));

import * as api from '../services/apiService.ts';
import { useTableActions } from '../hooks/useTableActions';

const fetchSpy = vi.spyOn(api, 'fetchL2Tps').mockResolvedValue({
  data: [],
  badRequest: false,
  error: null,
});

function Test() {
  const { openGenericTable } = useTableActions('1h', () => {}, null);
  openGenericTable('l2-tps');
  return null;
}

describe('openGenericTable', () => {
  beforeEach(() => {
    vi.stubGlobal('window', { location: mockLocation, history: { length: 1 } });
    navSpy.mockClear();
    fetchSpy.mockClear();
  });

  it('navigates without fetching when called from dashboard', () => {
    renderToStaticMarkup(React.createElement(Test));
    expect(navSpy).toHaveBeenCalledWith('l2-tps', { range: '1h' }, '1h');
    expect(fetchSpy).not.toHaveBeenCalled();
  });
});
