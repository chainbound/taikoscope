import { useCallback, useEffect, useState } from 'react';
import {
  useSearchParams as useRouterSearchParams,
  useNavigate,
} from 'react-router-dom';
import { safeNavigate } from '../utils/navigationUtils';

interface NavigationState {
  canGoBack: boolean;
  isNavigating: boolean;
  errorCount: number;
  lastError?: string;
}

export const useSearchParams = (): URLSearchParams & {
  navigate: (url: string | URL, replace?: boolean) => void;
  goBack: () => void;
  navigationState: NavigationState;
  resetNavigation: () => void;
} => {
  const [params] = useRouterSearchParams();
  const routerNavigate = useNavigate();
  const [navigationState, setNavigationState] = useState<NavigationState>({
    canGoBack:
      typeof window !== 'undefined' ? window.history.length > 1 : false,
    isNavigating: false,
    errorCount: 0,
    lastError: undefined,
  });

  const navigate = useCallback(
    (url: string | URL, replace = false) => {
      setNavigationState((prev) => ({ ...prev, isNavigating: true }));
      try {
        safeNavigate(routerNavigate, url, replace);
      } catch (err) {
        console.error('Navigation error:', err);
        setNavigationState((prev) => ({
          ...prev,
          errorCount: prev.errorCount + 1,
          lastError: 'Navigation failed',
        }));
      } finally {
        setNavigationState((prev) => ({
          ...prev,
          isNavigating: false,
          canGoBack:
            typeof window !== 'undefined' ? window.history.length > 1 : false,
        }));
      }
    },
    [routerNavigate],
  );

  const goBack = useCallback(() => {
    routerNavigate(-1);
  }, [routerNavigate]);

  const resetNavigation = useCallback(() => {
    safeNavigate(routerNavigate, '/', true);
  }, [routerNavigate]);

  useEffect(() => {
    if (typeof window === 'undefined') return;
    const handlePop = () => {
      setNavigationState((prev) => ({
        ...prev,
        canGoBack: window.history.length > 1,
      }));
    };
    window.addEventListener('popstate', handlePop);
    return () => {
      window.removeEventListener('popstate', handlePop);
    };
  }, []);

  return Object.assign(params, {
    navigate,
    goBack,
    navigationState,
    resetNavigation,
  });
};
