export const TOAST_EVENT = 'toast-event';

export const showToast = (message: string) => {
  if (typeof window === 'undefined' || !message) {
    return;
  }
  window.dispatchEvent(new CustomEvent(TOAST_EVENT, { detail: message }));
};
