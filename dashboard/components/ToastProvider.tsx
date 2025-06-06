import React, { useEffect, useState } from 'react';
import { TOAST_EVENT } from '../utils/toast';

interface Toast {
  id: number;
  message: string;
}

export const ToastProvider: React.FC<React.PropsWithChildren> = ({
  children,
}) => {
  const [toasts, setToasts] = useState<Toast[]>([]);

  useEffect(() => {
    const handler = (event: Event) => {
      const message = (event as CustomEvent<string>).detail;
      const id = Date.now() + Math.random();
      const toast: Toast = { id, message };
      setToasts((t) => [...t, toast]);
      setTimeout(() => {
        setToasts((t) => t.filter((x) => x.id !== id));
      }, 5000);
    };
    window.addEventListener(TOAST_EVENT, handler);
    return () => window.removeEventListener(TOAST_EVENT, handler);
  }, []);

  return (
    <>
      {children}
      <div className="fixed bottom-4 right-4 space-y-2 z-50">
        {toasts.map((t) => (
          <div
            key={t.id}
            className="bg-gray-800 text-white px-3 py-2 rounded shadow"
          >
            {t.message}
          </div>
        ))}
      </div>
    </>
  );
};
