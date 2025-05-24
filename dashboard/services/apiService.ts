export const API_BASE =
  (import.meta as any).env.VITE_API_BASE ||
  (import.meta as any).env.API_BASE ||
  "";

const fetchJson = async <T>(url: string): Promise<T | null> => {
  try {
    const res = await fetch(url);
    if (!res.ok) return null;
    return (await res.json()) as T;
  } catch {
    return null;
  }
};

export interface AvgTimeResponse {
  avg_prove_time_ms?: number;
  avg_verify_time_ms?: number;
}

export const fetchAvgProveTime = async (
  range: "1h" | "24h",
): Promise<number | null> => {
  const url =
    range === "1h"
      ? `${API_BASE}/avg-prove-time`
      : `${API_BASE}/avg-prove-time/24h`;
  const data = await fetchJson<{ avg_prove_time_ms?: number }>(url);
  return data?.avg_prove_time_ms ?? null;
};

export const fetchAvgVerifyTime = async (
  range: "1h" | "24h",
): Promise<number | null> => {
  const url =
    range === "1h"
      ? `${API_BASE}/avg-verify-time`
      : `${API_BASE}/avg-verify-time/24h`;
  const data = await fetchJson<{ avg_verify_time_ms?: number }>(url);
  return data?.avg_verify_time_ms ?? null;
};

export const fetchL2BlockCadence = async (
  range: "1h" | "24h",
): Promise<number | null> => {
  const url =
    range === "1h"
      ? `${API_BASE}/l2-block-cadence`
      : `${API_BASE}/l2-block-cadence/24h`;
  const data = await fetchJson<{ l2_block_cadence_ms?: number }>(url);
  return data?.l2_block_cadence_ms ?? null;
};

export const fetchBatchPostingCadence = async (
  range: "1h" | "24h",
): Promise<number | null> => {
  const url =
    range === "1h"
      ? `${API_BASE}/batch-posting-cadence`
      : `${API_BASE}/batch-posting-cadence/24h`;
  const data = await fetchJson<{ batch_posting_cadence_ms?: number }>(url);
  return data?.batch_posting_cadence_ms ?? null;
};

export const fetchActiveGateways = async (
  range: "1h" | "24h",
): Promise<number | null> => {
  const url =
    range === "1h"
      ? `${API_BASE}/active-gateways/last-hour`
      : `${API_BASE}/active-gateways/last-day`;
  const data = await fetchJson<{ gateways: string[] }>(url);
  return data ? data.gateways.length : null;
};

export const fetchL2Reorgs = async (
  range: "1h" | "24h",
): Promise<number | null> => {
  const url =
    range === "1h"
      ? `${API_BASE}/reorgs/last-hour`
      : `${API_BASE}/reorgs/last-day`;
  const data = await fetchJson<{ events: unknown[] }>(url);
  return data ? data.events.length : null;
};

export const fetchSlashingEvents = async (
  range: "1h" | "24h",
): Promise<number | null> => {
  const url =
    range === "1h"
      ? `${API_BASE}/slashings/last-hour`
      : `${API_BASE}/slashings/last-day`;
  const data = await fetchJson<{ events: unknown[] }>(url);
  return data ? data.events.length : null;
};

export const fetchForcedInclusions = async (
  range: "1h" | "24h",
): Promise<number | null> => {
  const url =
    range === "1h"
      ? `${API_BASE}/forced-inclusions/last-hour`
      : `${API_BASE}/forced-inclusions/last-day`;
  const data = await fetchJson<{ events: unknown[] }>(url);
  return data ? data.events.length : null;
};

export const fetchL2HeadBlock = async (
  range: "1h" | "24h",
): Promise<number | null> => {
  const url =
    range === "1h"
      ? `${API_BASE}/l2-block-times/last-hour`
      : `${API_BASE}/l2-block-times/last-day`;
  const data = await fetchJson<{ blocks: { block_number: number }[] }>(url);
  return data && data.blocks.length > 0
    ? data.blocks[data.blocks.length - 1].block_number
    : null;
};

export const fetchL1HeadBlock = async (
  range: "1h" | "24h",
): Promise<number | null> => {
  const url =
    range === "1h"
      ? `${API_BASE}/l1-block-times/last-hour`
      : `${API_BASE}/l1-block-times/last-day`;
  const data = await fetchJson<{ blocks: { block_number: number }[] }>(url);
  return data && data.blocks.length > 0
    ? data.blocks[data.blocks.length - 1].block_number
    : null;
};
