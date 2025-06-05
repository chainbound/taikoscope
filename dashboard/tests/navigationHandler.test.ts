import { describe, it, expect, beforeEach, vi } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';

const navigateMock = vi.fn();
const setSearchParamsMock = vi.fn();

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return {
    ...actual,
    useNavigate: () => navigateMock,
    useSearchParams: () => [
      new URLSearchParams('view=table&table=missed-proposals&range=1h'),
      setSearchParamsMock,
    ],
  };
});

import { useNavigationHandler } from '../hooks/useNavigationHandler';

describe('useNavigationHandler', () => {
  beforeEach(() => {
    navigateMock.mockClear();
    setSearchParamsMock.mockClear();
    vi.stubGlobal('window', { history: { length: 1 } });
  });

  it('navigates to dashboard when history lacks entries', () => {
    let handleBack: () => void = () => {};
    const Test = () => {
      handleBack = useNavigationHandler({
        setTableView: vi.fn(),
        onError: vi.fn(),
      }).handleBack;
      return null;
    };
    renderToStaticMarkup(React.createElement(Test));
    handleBack();
    expect(navigateMock).toHaveBeenCalledWith('/', { replace: true });
    expect(setSearchParamsMock).toHaveBeenCalledWith({}, { replace: true });
  });
});
