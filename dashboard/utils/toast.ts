export const TOAST_EVENT = 'toast-event';

export const showToast = (message: string) => {
  window.dispatchEvent(new CustomEvent(TOAST_EVENT, { detail: message }));
};
