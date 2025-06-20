import { useEffect, useState } from 'react';

export const useIsMobile = (breakpoint: number = 768): boolean => {
  const [isMobile, setIsMobile] = useState(
    typeof window !== 'undefined' && window.innerWidth < breakpoint,
  );

  useEffect(() => {
    const handleResize = () => {
      if (typeof window !== 'undefined') {
        setIsMobile(window.innerWidth < breakpoint);
      }
    };
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, [breakpoint]);

  return isMobile;
};
