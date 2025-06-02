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
    return () => window.removeEventListener('popstate', handleChange);
  }, [getParams]);

  return params;
};
