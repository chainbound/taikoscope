import { useCallback, useEffect, useState } from 'react';

export interface ApiQueryResult<T> {
  data: T | null;
  badRequest: boolean;
}

const cache = new Map<string, ApiQueryResult<unknown>>();

export function useApiQuery<T>(
  key: string,
  fetcher: () => Promise<ApiQueryResult<T>>,
) {
  const [result, setResult] = useState<ApiQueryResult<T> | null>(
    () => cache.get(key) as ApiQueryResult<T> | null,
  );
  const [loading, setLoading] = useState<boolean>(!cache.has(key));
  const [error, setError] = useState<unknown>(null);

  const execute = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetcher();
      cache.set(key, res as ApiQueryResult<unknown>);
      setResult(res);
    } catch (err) {
      setError(err);
    } finally {
      setLoading(false);
    }
  }, [key, fetcher]);

  useEffect(() => {
    if (!cache.has(key)) {
      void execute();
    } else {
      setResult(cache.get(key) as ApiQueryResult<T>);
    }
  }, [key, execute]);

  return { ...(result ?? { data: null, badRequest: false }), loading, error, refetch: execute };
}
