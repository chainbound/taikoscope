import { useEffect, useState } from 'react';

export const useIsMobile = (
  maxWidth: number = 640,
  landscapeHeight: number = 450,
): boolean => {
  const [isMobile, setIsMobile] = useState(() => {
    if (typeof window === 'undefined') {
      return false;
    }
    return (
      window.innerWidth <= maxWidth ||
      window.innerHeight <= landscapeHeight
    );
  });

  useEffect(() => {
    if (typeof window === 'undefined') {
      return;
    }
    const query = `(max-width: ${maxWidth}px), (max-height: ${landscapeHeight}px)`;
    const mql = window.matchMedia(query);
    const handler = (e: MediaQueryListEvent) => setIsMobile(e.matches);
    setIsMobile(mql.matches);
    mql.addEventListener('change', handler);
    return () => mql.removeEventListener('change', handler);
  }, [maxWidth, landscapeHeight]);

  return isMobile;
};
