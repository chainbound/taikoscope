import { API_BASE } from './services/apiService';

export interface BlockUpdateOptions {
  onL1Message: (value: string) => void;
  onL2Message: (value: string) => void;
  updateHeads: () => void | Promise<void>;
  setErrorMessage: (msg: string) => void;
  eventSourceFactory?: (url: string) => EventSource;
  setIntervalFn?: (fn: () => void, delay: number) => unknown;
  clearIntervalFn?: (id: unknown) => void;
}

export const setupBlockNumberUpdates = (
  options: BlockUpdateOptions,
) => {
  const {
    onL1Message,
    onL2Message,
    updateHeads,
    setErrorMessage,
    eventSourceFactory = (url: string) => new EventSource(url),
    setIntervalFn = setInterval,
    clearIntervalFn = clearInterval,
  } = options;

  let pollId: ReturnType<typeof setInterval> | null = null;

  const startPolling = () => {
    if (!pollId) {
      setErrorMessage('Realtime updates unavailable, falling back to polling.');
      updateHeads();
      pollId = setIntervalFn(updateHeads, 10000);
    }
  };

  const l1Source = eventSourceFactory(`${API_BASE}/sse/l1-head`);
  const l2Source = eventSourceFactory(`${API_BASE}/sse/l2-head`);

  l1Source.onmessage = (e) => onL1Message(Number(e.data).toLocaleString());
  l2Source.onmessage = (e) => onL2Message(Number(e.data).toLocaleString());

  const handleError = () => {
    l1Source.close();
    l2Source.close();
    startPolling();
  };

  l1Source.onerror = handleError;
  l2Source.onerror = handleError;

  return () => {
    l1Source.close();
    l2Source.close();
    if (pollId) clearIntervalFn(pollId);
  };
};
