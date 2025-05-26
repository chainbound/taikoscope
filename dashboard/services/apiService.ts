export const API_BASE =
  (import.meta as any).env.VITE_API_BASE ||
  (import.meta as any).env.API_BASE ||
  "";

export interface RequestResult<T> {
  data: T | null;
  badRequest: boolean;
}

const fetchJson = async <T>(url: string): Promise<RequestResult<T>> => {
  try {
    const res = await fetch(url);
    if (!res.ok) {
      return { data: null, badRequest: res.status === 400 };
    }
    return { data: (await res.json()) as T, badRequest: false };
  } catch {
    return { data: null, badRequest: false };
  }
};

export interface AvgTimeResponse {
  avg_prove_time_ms?: number;
  avg_verify_time_ms?: number;
}

export const fetchAvgProveTime = async (
  range: "1h" | "24h" | "7d",
): Promise<RequestResult<number>> => {
  const url =
    range === "1h"
      ? `${API_BASE}/avg-prove-time`
      : `${API_BASE}/avg-prove-time/24h`;
  const res = await fetchJson<{ avg_prove_time_ms?: number }>(url);
  return {
    data: res.data?.avg_prove_time_ms ?? null,
    badRequest: res.badRequest,
  };
};

export const fetchAvgVerifyTime = async (
  range: "1h" | "24h" | "7d",
): Promise<RequestResult<number>> => {
  const url =
    range === "1h"
      ? `${API_BASE}/avg-verify-time`
      : `${API_BASE}/avg-verify-time/24h`;
  const res = await fetchJson<{ avg_verify_time_ms?: number }>(url);
  return {
    data: res.data?.avg_verify_time_ms ?? null,
    badRequest: res.badRequest,
  };
};

export const fetchL2BlockCadence = async (
  range: "1h" | "24h" | "7d",
): Promise<RequestResult<number>> => {
  const url =
    range === "1h"
      ? `${API_BASE}/l2-block-cadence`
      : `${API_BASE}/l2-block-cadence/24h`;
  const res = await fetchJson<{ l2_block_cadence_ms?: number }>(url);
  return {
    data: res.data?.l2_block_cadence_ms ?? null,
    badRequest: res.badRequest,
  };
};

export const fetchBatchPostingCadence = async (
  range: "1h" | "24h" | "7d",
): Promise<RequestResult<number>> => {
  const url =
    range === "1h"
      ? `${API_BASE}/batch-posting-cadence`
      : `${API_BASE}/batch-posting-cadence/24h`;
  const res = await fetchJson<{ batch_posting_cadence_ms?: number }>(url);
  return {
    data: res.data?.batch_posting_cadence_ms ?? null,
    badRequest: res.badRequest,
  };
};

export const fetchActiveGateways = async (
  range: "1h" | "24h" | "7d",
): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/active-gateways?range=${range}`;
  const res = await fetchJson<{ gateways: string[] }>(url);
  return {
    data: res.data ? res.data.gateways.length : null,
    badRequest: res.badRequest,
  };
};

export const fetchL2Reorgs = async (
  range: "1h" | "24h" | "7d",
): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/reorgs?range=${range}`;
  const res = await fetchJson<{ events: unknown[] }>(url);
  return {
    data: res.data ? res.data.events.length : null,
    badRequest: res.badRequest,
  };
};

export const fetchSlashingEvents = async (
  range: "1h" | "24h" | "7d",
): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/slashings?range=${range}`;
  const res = await fetchJson<{ events: unknown[] }>(url);
  return {
    data: res.data ? res.data.events.length : null,
    badRequest: res.badRequest,
  };
};

export const fetchForcedInclusions = async (
  range: "1h" | "24h" | "7d",
): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/forced-inclusions?range=${range}`;
  const res = await fetchJson<{ events: unknown[] }>(url);
  return {
    data: res.data ? res.data.events.length : null,
    badRequest: res.badRequest,
  };
};

export const fetchL2HeadBlock = async (
  range: "1h" | "24h" | "7d",
): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/l2-block-times?range=${range}`;
  const res = await fetchJson<{ blocks: { block_number: number }[] }>(url);
  const value =
    res.data && res.data.blocks.length > 0
      ? res.data.blocks[res.data.blocks.length - 1].block_number
      : null;
  return { data: value, badRequest: res.badRequest };
};

export const fetchL1HeadBlock = async (
  range: "1h" | "24h" | "7d",
): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/l1-block-times?range=${range}`;
  const res = await fetchJson<{ blocks: { block_number: number }[] }>(url);
  const value =
    res.data && res.data.blocks.length > 0
      ? res.data.blocks[res.data.blocks.length - 1].block_number
      : null;
  return { data: value, badRequest: res.badRequest };
};
