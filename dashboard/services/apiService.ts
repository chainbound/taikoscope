const metaEnv = import.meta.env as any;
export const API_BASE: string =
  (metaEnv.VITE_API_BASE ?? metaEnv.API_BASE ?? '') + '/v1';

import { getSequencerName } from '../sequencerConfig';

import type {
  TimeSeriesData,
  PieChartDataItem,
  L2ReorgEvent,
  SlashingEvent,
  ForcedInclusionEvent,
  ErrorResponse,
} from '../types';

export interface RequestResult<T> {
  data: T | null;
  badRequest: boolean;
  error: ErrorResponse | null;
}

const fetchJson = async <T>(url: string): Promise<RequestResult<T>> => {
  try {
    const res = await fetch(url);
    if (!res.ok) {
      let error: ErrorResponse | null = null;
      try {
        error = (await res.json()) as ErrorResponse;
      } catch {
        // ignore JSON parse errors
      }
      return { data: null, badRequest: res.status === 400, error };
    }
    return { data: (await res.json()) as T, badRequest: false, error: null };
  } catch (error: unknown) {
    console.error(`Failed to fetch ${url}`, error);
    throw error;
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
    error: res.error,
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
    error: res.error,
  };
};

export const fetchL2BlockCadence = async (
  range: '1h' | '24h' | '7d',
  address?: string,
): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/l2-block-cadence?range=${range}${address ? `&address=${address}` : ''}`;
  const res = await fetchJson<{ l2_block_cadence_ms?: number }>(url);
  return {
    data: res.data?.l2_block_cadence_ms ?? null,
    badRequest: res.badRequest,
    error: res.error,
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
    error: res.error,
  };
};

export const fetchActiveSequencerAddresses = async (): Promise<RequestResult<string[]>> => {
  const res = await fetchPreconfData();
  return {
    data: res.data?.candidates ?? null,
    badRequest: res.badRequest,
    error: res.error,
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
    error: res.error,
  };
};

export const fetchL2ReorgEvents = async (
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<L2ReorgEvent[]>> => {
  const url = `${API_BASE}/reorgs?range=${range}`;
  const res = await fetchJson<{
    events: { l2_block_number: number; depth: number; inserted_at: string }[];
  }>(url);
  return {
    data: res.data
      ? res.data.events.map((e) => ({
          l2_block_number: e.l2_block_number,
          depth: e.depth,
          timestamp: Date.parse(e.inserted_at),
        }))
      : null,
    badRequest: res.badRequest,
    error: res.error,
  };
};

export const fetchSlashingEventCount = async (
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/slashings?range=${range}`;
  const res = await fetchJson<{ events: unknown[] }>(url);
  return {
    data: res.data ? res.data.events.length : null,
    badRequest: res.badRequest,
    error: res.error,
  };
};

export const fetchForcedInclusionCount = async (
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/forced-inclusions?range=${range}`;
  const res = await fetchJson<{ events: unknown[] }>(url);
  return {
    data: res.data ? res.data.events.length : null,
    badRequest: res.badRequest,
    error: res.error,
  };
};

export const fetchSlashingEvents = async (
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<SlashingEvent[]>> => {
  const url = `${API_BASE}/slashings?range=${range}`;
  const res = await fetchJson<{ events: SlashingEvent[] }>(url);
  return {
    data: res.data ? res.data.events : null,
    badRequest: res.badRequest,
    error: res.error,
  };
};

export const fetchForcedInclusionEvents = async (
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<ForcedInclusionEvent[]>> => {
  const url = `${API_BASE}/forced-inclusions?range=${range}`;
  const res = await fetchJson<{ events: ForcedInclusionEvent[] }>(url);
  return {
    data: res.data ? res.data.events : null,
    badRequest: res.badRequest,
    error: res.error,
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
  return { data: value, badRequest: res.badRequest, error: res.error };
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
  return { data: value, badRequest: res.badRequest, error: res.error };
};

export interface PreconfData {
  candidates: string[];
  current_operator?: string;
  next_operator?: string;
}

export const fetchPreconfData = async (): Promise<
  RequestResult<PreconfData>
> => {
  const url = `${API_BASE}/preconf-data`;
  return fetchJson<PreconfData>(url);
};

export const fetchL2HeadNumber = async (): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/l2-head-block`;
  const res = await fetchJson<{ l2_head_block?: number }>(url);
  return {
    data: res.data?.l2_head_block ?? null,
    badRequest: res.badRequest,
    error: res.error,
  };
};

export const fetchL1HeadNumber = async (): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/l1-head-block`;
  const res = await fetchJson<{ l1_head_block?: number }>(url);
  return {
    data: res.data?.l1_head_block ?? null,
    badRequest: res.badRequest,
    error: res.error,
  };
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
    error: res.error,
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
    error: res.error,
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
    return { data: null, badRequest: res.badRequest, error: res.error };
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

  return { data, badRequest: res.badRequest, error: res.error };
};

export const fetchL2BlockTimes = async (
  range: '1h' | '24h' | '7d',
  address?: string,
): Promise<RequestResult<TimeSeriesData[]>> => {
  const url = `${API_BASE}/l2-block-times?range=${range}${address ? `&address=${address}` : ''}`;
  const res = await fetchJson<{
    blocks: { l2_block_number: number; ms_since_prev_block: number }[];
  }>(url);
  if (!res.data) {
    return { data: null, badRequest: res.badRequest, error: res.error };
  }

  const data = res.data.blocks.slice(1).map(
    (b): TimeSeriesData => ({
      value: b.l2_block_number,
      timestamp: b.ms_since_prev_block,
    }),
  );

  return { data, badRequest: res.badRequest, error: res.error };
};

export const fetchBatchPostingTimes = async (
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<TimeSeriesData[]>> => {
  const url = `${API_BASE}/batch-posting-times?range=${range}`;
  const res = await fetchJson<{
    batches: { batch_id: number; ms_since_prev_batch: number }[];
  }>(url);
  if (!res.data) {
    return { data: null, badRequest: res.badRequest, error: res.error };
  }
  const data = res.data.batches.map(
    (b): TimeSeriesData => ({
      value: b.batch_id,
      timestamp: b.ms_since_prev_batch,
    }),
  );
  return { data, badRequest: res.badRequest, error: res.error };
};

export const fetchL2GasUsed = async (
  range: '1h' | '24h' | '7d',
  address?: string,
): Promise<RequestResult<TimeSeriesData[]>> => {
  const url = `${API_BASE}/l2-gas-used?range=${range}${address ? `&address=${address}` : ''}`;
  const res = await fetchJson<{
    blocks: { l2_block_number: number; gas_used: number }[];
  }>(url);
  if (!res.data) {
    return { data: null, badRequest: res.badRequest, error: res.error };
  }

  const data = res.data.blocks.slice(1).map(
    (b): TimeSeriesData => ({
      value: b.l2_block_number,
      timestamp: b.gas_used,
    }),
  );

  return { data, badRequest: res.badRequest, error: res.error };
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
      ? res.data.sequencers.map((s) => ({
          name: getSequencerName(s.address),
          value: s.blocks,
        }))
      : null,
    badRequest: res.badRequest,
    error: res.error,
  };
};

export const fetchSequencerBlocks = async (
  range: '1h' | '24h' | '7d',
  address: string,
): Promise<RequestResult<number[]>> => {
  const url = `${API_BASE}/sequencer-blocks?range=${range}&address=${address}`;
  const res = await fetchJson<{
    sequencers: { address: string; blocks: number[] }[];
  }>(url);
  const blocks = res.data?.sequencers.find(
    (s) => s.address.toLowerCase() === address.toLowerCase(),
  )?.blocks;
  return { data: blocks ?? null, badRequest: res.badRequest, error: res.error };
};

export interface BlockTransaction {
  block: number;
  txs: number;
  sequencer: string;
}

export const fetchBlockTransactions = async (
  range: '1h' | '24h' | '7d',
  limit = 50,
  startingAfter?: number,
  endingBefore?: number,
  address?: string,
): Promise<RequestResult<BlockTransaction[]>> => {
  let url = `${API_BASE}/block-transactions?range=${range}&limit=${limit}`;
  if (startingAfter !== undefined) {
    url += `&starting_after=${startingAfter}`;
  } else if (endingBefore !== undefined) {
    url += `&ending_before=${endingBefore}`;
  }
  if (address) {
    url += `&address=${address}`;
  }
  const res = await fetchJson<{ blocks: BlockTransaction[] }>(url);
  return {
    data: res.data?.blocks
      ? res.data.blocks.map((b) => ({
          ...b,
          sequencer: getSequencerName(b.sequencer),
        }))
      : null,
    badRequest: res.badRequest,
    error: res.error,
  };
};

export interface BatchBlobCount {
  batch: number;
  blobs: number;
}

export const fetchBatchBlobCounts = async (
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<BatchBlobCount[]>> => {
  const url = `${API_BASE}/blobs-per-batch?range=${range}`;
  const res = await fetchJson<{
    batches: { batch_id: number; blob_count: number }[];
  }>(url);
  return {
    data: res.data
      ? res.data.batches.map((b) => ({
          batch: b.batch_id,
          blobs: b.blob_count,
        }))
      : null,
    badRequest: res.badRequest,
    error: res.error,
  };
};

export const fetchAvgBlobsPerBatch = async (
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/avg-blobs-per-batch?range=${range}`;
  const res = await fetchJson<{ avg_blobs?: number }>(url);
  return {
    data: res.data?.avg_blobs ?? null,
    badRequest: res.badRequest,
    error: res.error,
  };
};

export const fetchAvgL2Tps = async (
  range: '1h' | '24h' | '7d',
  address?: string,
): Promise<RequestResult<number>> => {
  const url =
    `${API_BASE}/avg-l2-tps?range=${range}` +
    (address ? `&address=${address}` : '');
  const res = await fetchJson<{ avg_tps?: number }>(url);
  return {
    data: res.data?.avg_tps ?? null,
    badRequest: res.badRequest,
    error: res.error,
  };
};

export const fetchL2TxFee = async (
  range: '1h' | '24h' | '7d',
  address?: string,
): Promise<RequestResult<number>> => {
  const url =
    `${API_BASE}/l2-tx-fee?range=${range}` +
    (address ? `&address=${address}` : '');
  const res = await fetchJson<{ tx_fee?: number }>(url);
  return {
    data: res.data?.tx_fee ?? null,
    badRequest: res.badRequest,
    error: res.error,
  };
};

export const fetchCloudCost = async (
  range: '1h' | '24h' | '7d',
): Promise<RequestResult<number>> => {
  const url = `${API_BASE}/cloud-cost?range=${range}`;
  const res = await fetchJson<{ cost_usd: number }>(url);
  return {
    data: res.data?.cost_usd ?? null,
    badRequest: res.badRequest,
    error: res.error,
  };
};
