import { TimeRange } from '../types';
import { getSequencerAddress } from '../sequencerConfig';
import {
  fetchDashboardData,
  fetchProveTimes,
  fetchVerifyTimes,
  fetchL2BlockTimes,
  fetchL2GasUsed,
  fetchSequencerDistribution,
  fetchAllBlockTransactions,
  fetchBatchBlobCounts,
  fetchL2TxFee,
  fetchL2HeadBlock,
  fetchL1HeadBlock,
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
  l2TxFee: number | null;
  cloudCost: number | null;
  badRequestResults: any[];
}

export interface EconomicsData {
  l2TxFee: number | null;
  l2Block: number | null;
  l1Block: number | null;
  badRequestResults: any[];
}

export const fetchMainDashboardData = async (
  timeRange: TimeRange,
  selectedSequencer: string | null,
): Promise<MainDashboardData> => {
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
    fetchDashboardData(timeRange, address),
    fetchProveTimes(timeRange),
    fetchVerifyTimes(timeRange),
    fetchL2BlockTimes(timeRange, address),
    fetchL2GasUsed(timeRange, address),
    fetchSequencerDistribution(timeRange),
    fetchAllBlockTransactions(timeRange, address),
    fetchBatchBlobCounts(timeRange),
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
    l2TxFee: data?.l2_tx_fee ?? null,
    cloudCost: data?.cloud_cost ?? null,
    badRequestResults: allResults,
  };
};

export const fetchEconomicsData = async (
  timeRange: TimeRange,
  selectedSequencer: string | null,
): Promise<EconomicsData> => {
  const [l2TxFeeRes, l2BlockRes, l1BlockRes] = await Promise.all([
    fetchL2TxFee(
      timeRange,
      selectedSequencer ? getSequencerAddress(selectedSequencer) : undefined,
    ),
    fetchL2HeadBlock(timeRange),
    fetchL1HeadBlock(timeRange),
  ]);

  return {
    l2TxFee: l2TxFeeRes.data,
    l2Block: l2BlockRes.data,
    l1Block: l1BlockRes.data,
    badRequestResults: [l2TxFeeRes, l2BlockRes, l1BlockRes],
  };
};
