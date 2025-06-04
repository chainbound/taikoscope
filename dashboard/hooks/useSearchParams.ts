import { useCallback, useEffect, useState } from 'react';
import { useSearchParams as useRouterSearchParams, useNavigate } from 'react-router-dom';

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
    canGoBack: window.history.length > 1,
    isNavigating: false,
    errorCount: 0,
    lastError: undefined,
  });

  const navigate = useCallback(
    (url: string | URL, replace = false) => {
      setNavigationState((prev) => ({ ...prev, isNavigating: true }));
      try {
        const target = url instanceof URL ? url.toString() : url;
        routerNavigate(target, { replace });
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
          canGoBack: window.history.length > 1,
        }));
      }
    },
    [routerNavigate],
  );

  const goBack = useCallback(() => {
    routerNavigate(-1);
  }, [routerNavigate]);

  const resetNavigation = useCallback(() => {
    routerNavigate('/', { replace: true });
  }, [routerNavigate]);

  useEffect(() => {
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

  return Object.assign(params, { navigate, goBack, navigationState, resetNavigation });
};
