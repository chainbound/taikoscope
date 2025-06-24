import { TimeRange } from '../types';
import { getSequencerAddress } from '../sequencerConfig';
import { normalizeTimeRange } from './timeRange';
import {
  fetchDashboardData,
  fetchProveTimes,
  fetchVerifyTimes,
  fetchL2BlockTimesAggregated,
  fetchL2GasUsedAggregated,
  fetchSequencerDistribution,
  fetchBlockTransactionsAggregated,
  fetchBatchBlobCounts,

  fetchBatchL2Fees,
  fetchBatchDashboardData,
} from '../services/apiService';

export interface MainDashboardData {
  l2Cadence: number | null;
  batchCadence: number | null;
  avgProve: number | null;
  avgVerify: number | null;
  avgTps: number | null;
  preconfData: any;
  l2Reorgs: number | null;
  slashings: number | null;
  forcedInclusions: number | null;
  l2Block: number | null;
  l1Block: number | null;
  proveTimes: any[];
  verifyTimes: any[];
  l2Times: any[];
  l2Gas: any[];
  sequencerDist: any[];
  txPerBlock: any[];
  blobsPerBatch: any[];
  priorityFee: number | null;
  baseFee: number | null;
  badRequestResults: any[];
}

export interface EconomicsData {
  priorityFee: number | null;
  baseFee: number | null;
  l1DataCost: number | null;
  l2Block: number | null;
  l1Block: number | null;
  sequencerDist: {
    name: string;
    address: string;
    value: number;
    tps: number | null;
  }[];
  badRequestResults: any[];
}

export const fetchMainDashboardData = async (
  timeRange: TimeRange,
  selectedSequencer: string | null,
): Promise<MainDashboardData> => {
  const normalizedRange = normalizeTimeRange(timeRange);
  const address = selectedSequencer
    ? getSequencerAddress(selectedSequencer)
    : undefined;

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
    fetchDashboardData(normalizedRange, address),
    fetchProveTimes(normalizedRange),
    fetchVerifyTimes(normalizedRange),
    fetchL2BlockTimesAggregated(normalizedRange, address),
    fetchL2GasUsedAggregated(normalizedRange, address),
    fetchSequencerDistribution(normalizedRange),
    fetchBlockTransactionsAggregated(normalizedRange, address),
    fetchBatchBlobCounts(normalizedRange),
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
    l2Block: data?.l2_block ?? null,
    l1Block: data?.l1_block ?? null,
    proveTimes: proveTimesRes.data || [],
    verifyTimes: verifyTimesRes.data || [],
    l2Times: l2TimesRes.data || [],
    l2Gas: l2GasUsedRes.data || [],
    sequencerDist: sequencerDistRes.data || [],
    txPerBlock: blockTxRes.data || [],
    blobsPerBatch: batchBlobCountsRes.data || [],
    priorityFee: data?.priority_fee ?? null,
    baseFee: data?.base_fee ?? null,
    badRequestResults: allResults,
  };
};

export const fetchEconomicsData = async (
  timeRange: TimeRange,
  selectedSequencer: string | null,
): Promise<EconomicsData> => {
  const normalizedRange = normalizeTimeRange(timeRange);
  const [batchL2FeesRes, batchDashboardRes, sequencerDistRes] = await Promise.all([
    fetchBatchL2Fees(
      normalizedRange,
      selectedSequencer ? getSequencerAddress(selectedSequencer) : undefined,
    ),
    fetchBatchDashboardData(normalizedRange, selectedSequencer ? getSequencerAddress(selectedSequencer) : undefined),
    fetchSequencerDistribution(normalizedRange),
  ]);

  return {
    priorityFee: batchL2FeesRes.data?.priority_fee ?? null,
    baseFee: batchL2FeesRes.data?.base_fee ?? null,
    l1DataCost: batchL2FeesRes.data?.l1_data_cost ?? null,
    l2Block: batchDashboardRes.data?.l2_block ?? null,
    l1Block: batchDashboardRes.data?.l1_block ?? null,
    sequencerDist: sequencerDistRes.data || [],
    badRequestResults: [batchL2FeesRes, batchDashboardRes, sequencerDistRes],
  };
};
