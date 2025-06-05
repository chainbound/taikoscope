import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { MemoryRouter } from 'react-router-dom';
import { useNavigationHandler } from '../hooks/useNavigationHandler';
import { useSearchParams } from '../hooks/useSearchParams';

vi.mock('../hooks/useSearchParams');

const mockUseSearchParams = useSearchParams as unknown as {
  mockReturnValue: (val: unknown) => void;
};

describe('useNavigationHandler', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('goes back when history is available', () => {
    const navigate = vi.fn();
    const goBack = vi.fn();
    mockUseSearchParams.mockReturnValue({
      navigate,
      goBack,
      navigationState: { canGoBack: true, isNavigating: false, errorCount: 0 },
    });

    const setTableView = vi.fn();
    let handleBack: (() => void) | undefined;
    const Test = () => {
      ({ handleBack } = useNavigationHandler({
        setTableView,
        onError: () => {},
      }));
      return null;
    };
    renderToStaticMarkup(
      React.createElement(MemoryRouter, null, React.createElement(Test)),
    );

    handleBack!();

    expect(goBack).toHaveBeenCalled();
    expect(setTableView).toHaveBeenCalledWith(null);
  });

  it('navigates home when cannot go back', () => {
    const navigate = vi.fn();
    const goBack = vi.fn();
    mockUseSearchParams.mockReturnValue({
      navigate,
      goBack,
      navigationState: { canGoBack: false, isNavigating: false, errorCount: 0 },
    });

    const setTableView = vi.fn();
    let handleBack: (() => void) | undefined;
    const Test = () => {
      ({ handleBack } = useNavigationHandler({
        setTableView,
        onError: () => {},
      }));
      return null;
    };
    vi.stubGlobal('window', {
      location: new URL('https://example.com/dashboard?view=table&page=1'),
    });
    renderToStaticMarkup(
      React.createElement(MemoryRouter, null, React.createElement(Test)),
    );

    handleBack!();

    expect(navigate).toHaveBeenCalledWith('/dashboard', true);
    expect(setTableView).toHaveBeenCalledWith(null);
  });
});
