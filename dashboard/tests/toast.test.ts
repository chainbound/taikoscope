import { describe, it, expect, vi, beforeEach } from 'vitest';
import { showToast, TOAST_EVENT } from '../utils/toast';

describe('toast', () => {
  beforeEach(() => {
    vi.stubGlobal('window', {
      dispatchEvent: vi.fn(),
    });
  });

  it('dispatches a custom event with the message', () => {
    const spy = vi.spyOn(window, 'dispatchEvent');
    showToast('hello');
    expect(spy).toHaveBeenCalledTimes(1);
    const event = spy.mock.calls[0][0] as CustomEvent;
    expect(event.type).toBe(TOAST_EVENT);
    expect(event.detail).toBe('hello');
    spy.mockRestore();
  });
});
