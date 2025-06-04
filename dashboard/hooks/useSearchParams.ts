import { useCallback, useEffect, useState } from 'react';

export const useSearchParams = (): URLSearchParams => {
  const getParams = useCallback(
    () => new URLSearchParams(window.location.search),
    [],
  );

  const [params, setParams] = useState<URLSearchParams>(getParams);

  useEffect(() => {
    const handleChange = () => setParams(getParams());
    window.addEventListener('popstate', handleChange);

    const { pushState, replaceState } = window.history;

    window.history.pushState = (
      ...args: Parameters<History['pushState']>
    ): void => {
      pushState.apply(window.history, args);
      window.dispatchEvent(new Event('popstate'));
    };

    window.history.replaceState = (
      ...args: Parameters<History['replaceState']>
    ): void => {
      replaceState.apply(window.history, args);
      window.dispatchEvent(new Event('popstate'));
    };

    return () => {
      window.removeEventListener('popstate', handleChange);
      window.history.pushState = pushState;
      window.history.replaceState = replaceState;
    };
  }, [getParams]);

  return params;
};
