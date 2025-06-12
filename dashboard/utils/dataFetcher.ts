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
  fetchL2Fees,
  fetchL2HeadBlock,
  fetchL1HeadBlock,
  fetchCloudCost,
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
  cloudCost: number | null;
  badRequestResults: any[];
}

export interface EconomicsData {
  priorityFee: number | null;
  baseFee: number | null;
  l2Block: number | null;
  l1Block: number | null;
  cloudCost: number | null;
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
  ] = await Promise.allSettled([
    fetchDashboardData(timeRange, address),
    fetchProveTimes(timeRange),
    fetchVerifyTimes(timeRange),
    fetchL2BlockTimes(timeRange, address),
    fetchL2GasUsed(timeRange, address),
    fetchSequencerDistribution(timeRange),
    fetchAllBlockTransactions(timeRange, address),
    fetchBatchBlobCounts(timeRange),
  ]);

  const getValue = (res: PromiseSettledResult<any>) =>
    res.status === 'fulfilled' ? res.value : null;

  const allResults = [
    dashboardRes,
    proveTimesRes,
    verifyTimesRes,
    l2TimesRes,
    l2GasUsedRes,
    sequencerDistRes,
    blockTxRes,
    batchBlobCountsRes,
  ].map(getValue);

  const [
    dashboardData,
    proveTimesData,
    verifyTimesData,
    l2TimesData,
    l2GasUsedData,
    sequencerDistData,
    blockTxData,
    batchBlobCountsData,
  ] = allResults;

  const data = dashboardData?.data;

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
    proveTimes: proveTimesData?.data || [],
    verifyTimes: verifyTimesData?.data || [],
    l2Times: l2TimesData?.data || [],
    l2Gas: l2GasUsedData?.data || [],
    sequencerDist: sequencerDistData?.data || [],
    txPerBlock: blockTxData?.data || [],
    blobsPerBatch: batchBlobCountsData?.data || [],
    priorityFee: data?.priority_fee ?? null,
    baseFee: data?.base_fee ?? null,
    cloudCost: data?.cloud_cost ?? null,
    badRequestResults: allResults.filter((res) => res && res.status !== 200),
  };
};

export const fetchEconomicsData = async (
  timeRange: TimeRange,
  selectedSequencer: string | null,
): Promise<EconomicsData> => {
  const [l2FeesRes, l2BlockRes, l1BlockRes, cloudCostRes] = await Promise.all([
    fetchL2Fees(
      timeRange,
      selectedSequencer ? getSequencerAddress(selectedSequencer) : undefined,
    ),
    fetchL2HeadBlock(timeRange),
    fetchL1HeadBlock(timeRange),
    fetchCloudCost(timeRange),
  ]);

  return {
    priorityFee: l2FeesRes.data?.priority_fee ?? null,
    baseFee: l2FeesRes.data?.base_fee ?? null,
    l2Block: l2BlockRes.data,
    l1Block: l1BlockRes.data,
    cloudCost: cloudCostRes.data,
    badRequestResults: [l2FeesRes, l2BlockRes, l1BlockRes, cloudCostRes],
  };
};
