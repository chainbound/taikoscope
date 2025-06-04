import { useCallback, useEffect, useState } from 'react';

interface NavigationState {
  canGoBack: boolean;
  isNavigating: boolean;
}

export const useSearchParams = (): URLSearchParams & {
  navigate: (url: string | URL, replace?: boolean) => void;
  goBack: () => void;
  navigationState: NavigationState;
} => {
  const getParams = useCallback(
    () => new URLSearchParams(window.location.search),
    [],
  );

  const [params, setParams] = useState<URLSearchParams>(getParams);
  const [navigationState, setNavigationState] = useState<NavigationState>({
    canGoBack: window.history.length > 1,
    isNavigating: false,
  });

  const navigate = useCallback(
    (url: string | URL, replace = false) => {
      if (navigationState.isNavigating) return;

      setNavigationState((prev) => ({ ...prev, isNavigating: true }));

      try {
        const urlString = url instanceof URL ? url.toString() : url;

        try {
          if (replace) {
            window.history.replaceState(null, '', urlString);
          } else {
            window.history.pushState(null, '', urlString);
          }

          window.dispatchEvent(new Event('popstate'));
        } catch (err) {
          // eslint-disable-next-line no-console
          console.error('Failed to update history:', err);
          window.location.assign(urlString);
          return;
        }
      } finally {
        setTimeout(() => {
          setNavigationState((prev) => ({
            ...prev,
            isNavigating: false,
            canGoBack: window.history.length > 1,
          }));
        }, 100);
      }
    },
    [navigationState.isNavigating],
  );

  const goBack = useCallback(() => {
    if (navigationState.isNavigating) return;

    if (window.history.length > 1) {
      setNavigationState((prev) => ({ ...prev, isNavigating: true }));
      window.history.back();
    } else {
      // Fallback: navigate to dashboard home
      const url = new URL(window.location.href);
      url.search = '';
      navigate(url.toString(), true);
    }
  }, [navigationState.isNavigating, navigate]);

  useEffect(() => {
    const handleChange = () => {
      setParams(getParams());
      setNavigationState((prev) => ({
        ...prev,
        canGoBack: window.history.length > 1,
        isNavigating: false,
      }));
    };

    const handleBeforeUnload = () => {
      setNavigationState((prev) => ({ ...prev, isNavigating: false }));
    };

    window.addEventListener('popstate', handleChange);
    window.addEventListener('beforeunload', handleBeforeUnload);

    const { pushState, replaceState } = window.history;

    window.history.pushState = (
      ...args: Parameters<History['pushState']>
    ): void => {
      if (args[2] instanceof URL) args[2] = args[2].toString();
      pushState.apply(window.history, args);
      window.dispatchEvent(new Event('popstate'));
    };

    window.history.replaceState = (
      ...args: Parameters<History['replaceState']>
    ): void => {
      if (args[2] instanceof URL) args[2] = args[2].toString();
      replaceState.apply(window.history, args);
      window.dispatchEvent(new Event('popstate'));
    };

    return () => {
      window.removeEventListener('popstate', handleChange);
      window.removeEventListener('beforeunload', handleBeforeUnload);
      window.history.pushState = pushState;
      window.history.replaceState = replaceState;
    };
  }, [getParams]);

  return Object.assign(params, { navigate, goBack, navigationState });
};
