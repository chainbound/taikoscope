export const API_BASE =
  (import.meta as any).env.VITE_API_BASE ||
  (import.meta as any).env.API_BASE ||
  "";

export interface AvgTimeResponse {
  avg_prove_time_ms?: number;
  avg_verify_time_ms?: number;
}

export const fetchAvgProveTime = async (): Promise<number | null> => {
  try {
    const res = await fetch(`${API_BASE}/avg-prove-time`);
    if (!res.ok) return null;
    const data: { avg_prove_time_ms?: number } = await res.json();
    return data.avg_prove_time_ms ?? null;
  } catch {
    return null;
  }
};

export const fetchAvgVerifyTime = async (): Promise<number | null> => {
  try {
    const res = await fetch(`${API_BASE}/avg-verify-time`);
    if (!res.ok) return null;
    const data: { avg_verify_time_ms?: number } = await res.json();
    return data.avg_verify_time_ms ?? null;
  } catch {
    return null;
  }
};
