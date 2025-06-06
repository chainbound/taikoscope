import React, { useEffect, useRef, useState } from 'react';

interface LazyMountProps {
  children: React.ReactNode;
}

/**
 * Mounts children only after the component scrolls into view
 * using an IntersectionObserver.
 */
export const LazyMount: React.FC<LazyMountProps> = ({ children }) => {
  const ref = useRef<HTMLDivElement | null>(null);
  const [isVisible, setIsVisible] = useState(
    () => typeof window === 'undefined',
  );

  useEffect(() => {
    const element = ref.current;
    if (!element || typeof window === 'undefined') {
      setIsVisible(true);
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting) {
          setIsVisible(true);
          observer.disconnect();
        }
      },
      { rootMargin: '200px' },
    );

    observer.observe(element);
    return () => {
      observer.disconnect();
    };
  }, []);

  return (
    <div ref={ref} className="w-full h-full">
      {isVisible ? children : null}
    </div>
  );
};
