import { useCallback, useEffect, useState } from 'react';
import { sanitizeUrl, createSafeUrl, validateSearchParams, cleanSearchParams } from '../utils/navigationUtils';

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
  const getParams = useCallback(
    () => {
      try {
        const params = new URLSearchParams(window.location.search);
        if (!validateSearchParams(params)) {
          console.warn('Invalid search parameters detected, cleaning...');
          return cleanSearchParams(params);
        }
        return params;
      } catch {
        return new URLSearchParams();
      }
    },
    [],
  );

  const [params, setParams] = useState<URLSearchParams>(getParams);
  const [navigationState, setNavigationState] = useState<NavigationState>({
    canGoBack: window.history.length > 1,
    isNavigating: false,
    errorCount: 0,
    lastError: undefined,
  });

  const navigate = useCallback(
    (url: string | URL, replace = false) => {
      if (navigationState.isNavigating) return;

      setNavigationState((prev) => ({ ...prev, isNavigating: true }));

      try {
        const urlString = url instanceof URL ? url.toString() : url;

        // Validate and sanitize URL
        const sanitizedUrl = sanitizeUrl(urlString);
        if (sanitizedUrl !== urlString) {
          console.warn('URL was sanitized:', { original: urlString, sanitized: sanitizedUrl });
        }

        try {
          if (replace) {
            window.history.replaceState(null, '', sanitizedUrl);
          } else {
            window.history.pushState(null, '', sanitizedUrl);
          }

          window.dispatchEvent(new Event('popstate'));
          
          // Clear error state on successful navigation
          setNavigationState((prev) => ({
            ...prev,
            errorCount: 0,
            lastError: undefined,
          }));
        } catch (err) {
          console.error('Failed to update history:', err);
          
          // Track navigation errors
          setNavigationState((prev) => ({
            ...prev,
            errorCount: prev.errorCount + 1,
            lastError: err instanceof Error ? err.message : 'Navigation failed',
          }));

          // Fallback to location.assign only if error count is low to prevent loops
          if (navigationState.errorCount < 3) {
            try {
              window.location.assign(sanitizedUrl);
            } catch (assignErr) {
              console.error('Failed to assign location:', assignErr);
              setNavigationState((prev) => ({
                ...prev,
                lastError: 'Complete navigation failure',
              }));
            }
          }
          return;
        }
      } catch (outerErr) {
        console.error('Unexpected navigation error:', outerErr);
        setNavigationState((prev) => ({
          ...prev,
          errorCount: prev.errorCount + 1,
          lastError: 'Unexpected navigation error',
        }));
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
    [navigationState.isNavigating, navigationState.errorCount],
  );

  const goBack = useCallback(() => {
    if (navigationState.isNavigating) return;

    setNavigationState((prev) => ({ ...prev, isNavigating: true }));

    try {
      if (window.history.length > 1) {
        // Add a timeout to detect if back() fails
        const backTimeout = setTimeout(() => {
          console.warn('Back navigation appears to have failed, using fallback');
          const url = new URL(window.location.href);
          url.search = '';
          navigate(url.toString(), true);
        }, 1000);

        const handleBackSuccess = () => {
          clearTimeout(backTimeout);
          window.removeEventListener('popstate', handleBackSuccess);
        };

        window.addEventListener('popstate', handleBackSuccess, { once: true });
        window.history.back();
      } else {
        // Fallback: navigate to dashboard home
        const url = createSafeUrl();
        url.search = '';
        navigate(sanitizeUrl(url), true);
      }
    } catch (err) {
      console.error('Failed to go back:', err);
      setNavigationState((prev) => ({
        ...prev,
        errorCount: prev.errorCount + 1,
        lastError: 'Back navigation failed',
      }));
      
      // Fallback to home
      const url = new URL(window.location.href);
      url.search = '';
      navigate(url.toString(), true);
    }
  }, [navigationState.isNavigating, navigate]);

  const resetNavigation = useCallback(() => {
    try {
      setNavigationState({
        canGoBack: window.history.length > 1,
        isNavigating: false,
        errorCount: 0,
        lastError: undefined,
      });
      
      // Clear URL parameters and go to clean dashboard state
      const url = createSafeUrl();
      url.search = '';
      const cleanUrl = sanitizeUrl(url);
      window.history.replaceState(null, '', cleanUrl);
      window.dispatchEvent(new Event('popstate'));
    } catch (err) {
      console.error('Failed to reset navigation:', err);
      // Last resort: reload the page
      window.location.href = window.location.pathname;
    }
  }, []);

  useEffect(() => {
    const handleChange = () => {
      try {
        setParams(getParams());
        setNavigationState((prev) => ({
          ...prev,
          canGoBack: window.history.length > 1,
          isNavigating: false,
        }));
      } catch (err) {
        console.error('Failed to handle navigation change:', err);
        setNavigationState((prev) => ({
          ...prev,
          isNavigating: false,
          errorCount: prev.errorCount + 1,
          lastError: 'Failed to parse URL parameters',
        }));
      }
    };

    const handleBeforeUnload = () => {
      setNavigationState((prev) => ({ 
        ...prev, 
        isNavigating: false,
        errorCount: 0,
        lastError: undefined,
      }));
    };

    window.addEventListener('popstate', handleChange);
    window.addEventListener('beforeunload', handleBeforeUnload);

    const { pushState, replaceState } = window.history;

    window.history.pushState = (
      ...args: Parameters<History['pushState']>
    ): void => {
      try {
        if (args[2] instanceof URL) args[2] = args[2].toString();
        pushState.apply(window.history, args);
        window.dispatchEvent(new Event('popstate'));
      } catch (err) {
        console.error('pushState failed:', err);
        // Don't throw to prevent breaking the application
      }
    };

    window.history.replaceState = (
      ...args: Parameters<History['replaceState']>
    ): void => {
      try {
        if (args[2] instanceof URL) args[2] = args[2].toString();
        replaceState.apply(window.history, args);
        window.dispatchEvent(new Event('popstate'));
      } catch (err) {
        console.error('replaceState failed:', err);
        // Don't throw to prevent breaking the application
      }
    };

    return () => {
      window.removeEventListener('popstate', handleChange);
      window.removeEventListener('beforeunload', handleBeforeUnload);
      window.history.pushState = pushState;
      window.history.replaceState = replaceState;
    };
  }, [getParams]);

  return Object.assign(params, { navigate, goBack, navigationState, resetNavigation });
};
