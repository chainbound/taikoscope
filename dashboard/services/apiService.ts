export const API_BASE =
  (import.meta as any).env.VITE_API_BASE ||
  (import.meta as any).env.API_BASE ||
  '';

import type { TimeSeriesData, PieChartDataItem } from '../types';

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
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/avg-prove-time?range=${range}`;
  const res = await fetchJson<{ avg_prove_time_ms?: number }>(url);
  return {
    data: res.data?.avg_prove_time_ms ?? null,
    badRequest: res.badRequest,
  };
};

export const fetchAvgVerifyTime = async (
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/avg-verify-time?range=${range}`;
  const res = await fetchJson<{ avg_verify_time_ms?: number }>(url);
  return {
    data: res.data?.avg_verify_time_ms ?? null,
    badRequest: res.badRequest,
  };
};

export const fetchL2BlockCadence = async (
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/l2-block-cadence?range=${range}`;
  const res = await fetchJson<{ l2_block_cadence_ms?: number }>(url);
  return {
    data: res.data?.l2_block_cadence_ms ?? null,
    badRequest: res.badRequest,
  };
};

export const fetchBatchPostingCadence = async (
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/batch-posting-cadence?range=${range}`;
  const res = await fetchJson<{ batch_posting_cadence_ms?: number }>(url);
  return {
    data: res.data?.batch_posting_cadence_ms ?? null,
    badRequest: res.badRequest,
  };
};

export const fetchActiveGateways = async (
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/active-gateways?range=${range}`;
  const res = await fetchJson<{ gateways: string[] }>(url);
  return {
    data: res.data ? res.data.gateways.length : null,
    badRequest: res.badRequest,
  };
};

export const fetchL2Reorgs = async (
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/reorgs?range=${range}`;
  const res = await fetchJson<{ events: unknown[] }>(url);
  return {
    data: res.data ? res.data.events.length : null,
    badRequest: res.badRequest,
  };
};

export const fetchSlashingEvents = async (
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/slashings?range=${range}`;
  const res = await fetchJson<{ events: unknown[] }>(url);
  return {
    data: res.data ? res.data.events.length : null,
    badRequest: res.badRequest,
  };
};

export const fetchForcedInclusions = async (
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/forced-inclusions?range=${range}`;
  const res = await fetchJson<{ events: unknown[] }>(url);
  return {
    data: res.data ? res.data.events.length : null,
    badRequest: res.badRequest,
  };
};

export const fetchL2HeadBlock = async (
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/l2-block-times?range=${range}`;
  const res = await fetchJson<{ blocks: { l2_block_number: number }[] }>(url);
  const value =
    res.data && res.data.blocks.length > 0
      ? res.data.blocks[res.data.blocks.length - 1].l2_block_number
      : null;
  return { data: value, badRequest: res.badRequest };
};

export const fetchL1HeadBlock = async (
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/l1-block-times?range=${range}`;
  const res = await fetchJson<{ blocks: { block_number: number }[] }>(url);
  const value =
    res.data && res.data.blocks.length > 0
      ? res.data.blocks[res.data.blocks.length - 1].block_number
      : null;
  return { data: value, badRequest: res.badRequest };
};

export const fetchCurrentOperator = async (): Promise<
  RequestResult<string>
> => {
  const url = `${API_BASE}/current-operator`;
  const res = await fetchJson<{ operator?: string }>(url);
  return { data: res.data?.operator ?? null, badRequest: res.badRequest };
};

export const fetchNextOperator = async (): Promise<RequestResult<string>> => {
  const url = `${API_BASE}/next-operator`;
  const res = await fetchJson<{ operator?: string }>(url);
  return { data: res.data?.operator ?? null, badRequest: res.badRequest };
};

export const fetchL2HeadNumber = async (): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/l2-head-block`;
  const res = await fetchJson<{ l2_head_block?: number }>(url);
  return { data: res.data?.l2_head_block ?? null, badRequest: res.badRequest };
};

export const fetchL1HeadNumber = async (): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/l1-head-block`;
  const res = await fetchJson<{ l1_head_block?: number }>(url);
  return { data: res.data?.l1_head_block ?? null, badRequest: res.badRequest };
};

export const fetchProveTimes = async (
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<TimeSeriesData[]>> => {
  const url = `${API_BASE}/prove-times?range=${range}`;
  const res = await fetchJson<{
    batches: { batch_id: number; seconds_to_prove: number }[];
  }>(url);
  return {
    data: res.data
      ? res.data.batches.map((b) => ({
          name: b.batch_id.toString(),
          value: b.seconds_to_prove,
          timestamp: 0,
        }))
      : null,
    badRequest: res.badRequest,
  };
};

export const fetchVerifyTimes = async (
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<TimeSeriesData[]>> => {
  const url = `${API_BASE}/verify-times?range=${range}`;
  const res = await fetchJson<{
    batches: { batch_id: number; seconds_to_verify: number }[];
  }>(url);
  return {
    data: res.data
      ? res.data.batches.map((b) => ({
          name: b.batch_id.toString(),
          value: b.seconds_to_verify,
          timestamp: 0,
        }))
      : null,
    badRequest: res.badRequest,
  };
};

export const fetchL1BlockTimes = async (
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<TimeSeriesData[]>> => {
  const url = `${API_BASE}/l1-block-times?range=${range}`;
  const res = await fetchJson<{
    blocks: { minute: number; block_number: number }[];
  }>(url);
  if (!res.data) {
    return { data: null, badRequest: res.badRequest };
  }

  const blocks = res.data.blocks.map((b) => ({
    ts: b.minute * 1000,
    block: b.block_number,
  }));

  const data = blocks
    .slice(1)
    .map((b, i): TimeSeriesData | null => {
      const prev = blocks[i];
      const deltaBlocks = b.block - prev.block;
      if (deltaBlocks <= 0) {
        return null;
      }
      const deltaTimeMs = b.ts - prev.ts;
      const interval = deltaTimeMs / deltaBlocks;
      return { timestamp: interval, value: b.block };
    })
    .filter((d): d is TimeSeriesData => d !== null);

  return { data, badRequest: res.badRequest };
};

export const fetchL2BlockTimes = async (
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<TimeSeriesData[]>> => {
  const url = `${API_BASE}/l2-block-times?range=${range}`;
  const res = await fetchJson<{
    blocks: { l2_block_number: number; seconds_since_prev_block: number }[];
  }>(url);
  if (!res.data) {
    return { data: null, badRequest: res.badRequest };
  }

  const data = res.data.blocks.slice(1).map(
    (b): TimeSeriesData => ({
      value: b.l2_block_number,
      timestamp: b.seconds_since_prev_block * 1000,
    }),
  );

  return { data, badRequest: res.badRequest };
};

export const fetchSequencerDistribution = async (
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<PieChartDataItem[]>> => {
  const url = `${API_BASE}/sequencer-distribution?range=${range}`;
  const res = await fetchJson<{
    sequencers: { address: string; blocks: number }[];
  }>(url);
  return {
    data: res.data
      ? res.data.sequencers.map((s) => ({ name: s.address, value: s.blocks }))
      : null,
    badRequest: res.badRequest,
  };
};
