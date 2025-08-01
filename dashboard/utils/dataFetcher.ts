import { TimeRange, type TimeSeriesData } from '../types';
import { getSequencerAddress, getSequencerName } from '../sequencerConfig';
import { normalizeTimeRange } from './timeRange';
import {
  fetchDashboardData,
  fetchProveTimesAggregated,
  fetchVerifyTimesAggregated,
  fetchL2BlockTimesAggregated,
  fetchL2GasUsedAggregated,
  fetchSequencerDistribution,
  fetchBlockTransactionsAggregated,
  fetchBatchBlobCountsAggregated,
  fetchL2Fees,
  fetchL2HeadBlock,
  fetchL1HeadBlock,
  type PreconfData,
  type SequencerDistributionDataItem,
  type BlockTransaction,
  type BatchBlobCount,
  type RequestResult,
} from '../services/apiService';

export interface MainDashboardData {
  l2Cadence: number | null;
  batchCadence: number | null;
  avgProve: number | null;
  avgVerify: number | null;
  avgTps: number | null;
  preconfData: PreconfData | null;
  l2Reorgs: number | null;
  slashings: number | null;
  forcedInclusions: number | null;
  failedProposals: number | null;
  l2Block: number | null;
  l1Block: number | null;
  proveTimes: TimeSeriesData[];
  verifyTimes: TimeSeriesData[];
  l2Times: TimeSeriesData[];
  l2Gas: TimeSeriesData[];
  sequencerDist: SequencerDistributionDataItem[];
  txPerBlock: BlockTransaction[];
  blobsPerBatch: BatchBlobCount[];
  badRequestResults: RequestResult<unknown>[];
}

export interface EconomicsData {
  priorityFee: number | null;
  baseFee: number | null;
  l1DataCost: number;
  proveCost: number;
  l2Block: number | null;
  l1Block: number | null;
  sequencerDist: SequencerDistributionDataItem[];
  badRequestResults: RequestResult<unknown>[];
}

export const fetchMainDashboardData = async (
  timeRange: TimeRange,
  selectedSequencer: string | null,
): Promise<MainDashboardData> => {
  const normalizedRange = normalizeTimeRange(timeRange);
  const address = selectedSequencer
    ? getSequencerAddress(selectedSequencer)
    : undefined;

  function safe<T>(pr: Promise<RequestResult<T>>): Promise<RequestResult<T>> {
    return pr.catch(() => ({ data: null, badRequest: false, error: null }));
  }

  const [
    dashboardRes,
    proveTimesRes,
    verifyTimesRes,
    l2TimesRes,
    l2GasUsedRes,
    sequencerDistRes,
    blockTxRes,
    batchBlobCountsRes,
  ] = await Promise.all([
    safe(fetchDashboardData(normalizedRange, address)),
    safe(fetchProveTimesAggregated(normalizedRange)),
    safe(fetchVerifyTimesAggregated(normalizedRange)),
    safe(fetchL2BlockTimesAggregated(normalizedRange, address)),
    safe(fetchL2GasUsedAggregated(normalizedRange, address)),
    safe(fetchSequencerDistribution(normalizedRange)),
    safe(fetchBlockTransactionsAggregated(normalizedRange, address)),
    safe(fetchBatchBlobCountsAggregated(normalizedRange)),
  ]);

  const data = dashboardRes.data;

  const allResults = [
    dashboardRes,
    proveTimesRes,
    verifyTimesRes,
    l2TimesRes,
    l2GasUsedRes,
    sequencerDistRes,
    blockTxRes,
    batchBlobCountsRes,
  ];

  return {
    l2Cadence: data?.l2_block_cadence_ms ?? null,
    batchCadence: data?.batch_posting_cadence_ms ?? null,
    avgProve: data?.avg_prove_time_ms ?? null,
    avgVerify: data?.avg_verify_time_ms ?? null,
    avgTps: data?.avg_tps ?? null,
    preconfData: data?.preconf_data ?? null,
    l2Reorgs: data?.l2_reorgs ?? null,
    slashings: data?.slashings ?? null,
    forcedInclusions: data?.forced_inclusions ?? null,
    failedProposals: data?.failed_proposals ?? null,
    l2Block: data?.l2_head_block ?? null,
    l1Block: data?.l1_head_block ?? null,
    proveTimes: proveTimesRes.data || [],
    verifyTimes: verifyTimesRes.data || [],
    l2Times: l2TimesRes.data || [],
    l2Gas: l2GasUsedRes.data || [],
    sequencerDist: sequencerDistRes.data || [],
    txPerBlock: blockTxRes.data || [],
    blobsPerBatch: batchBlobCountsRes.data || [],
    badRequestResults: allResults.slice(1),
  };
};

export const fetchEconomicsData = async (
  timeRange: TimeRange,
  selectedSequencer: string | null,
): Promise<EconomicsData> => {
  const normalizedRange = normalizeTimeRange(timeRange);
  const [l2FeesRes, l2BlockRes, l1BlockRes] = await Promise.all([
    fetchL2Fees(
      normalizedRange,
      selectedSequencer ? getSequencerAddress(selectedSequencer) : undefined,
    ),
    fetchL2HeadBlock(normalizedRange),
    fetchL1HeadBlock(normalizedRange),
  ]);

  const sequencerDist = l2FeesRes.data
    ? l2FeesRes.data.sequencers.map((s) => ({
        name: getSequencerName(s.address),
        address: s.address,
        value: 0,
        batches: 0,
        tps: null,
      }))
    : [];

  return {
    priorityFee: l2FeesRes.data?.priority_fee ?? null,
    baseFee: l2FeesRes.data?.base_fee ?? null,
    l1DataCost: l2FeesRes.data?.l1_data_cost ?? 0,
    proveCost: l2FeesRes.data?.prove_cost ?? 0,

    l2Block: l2BlockRes.data,
    l1Block: l1BlockRes.data,
    sequencerDist,
    badRequestResults: [l2FeesRes, l2BlockRes, l1BlockRes],
  };
};
