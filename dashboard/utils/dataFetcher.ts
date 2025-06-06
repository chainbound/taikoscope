import { TimeRange } from '../types';
import { getSequencerAddress } from '../sequencerConfig';
import {
  fetchAvgProveTime,
  fetchAvgVerifyTime,
  fetchL2BlockCadence,
  fetchBatchPostingCadence,
  fetchL2Reorgs,
  fetchSlashingEventCount,
  fetchForcedInclusionCount,
  fetchPreconfData,
  fetchL2HeadBlock,
  fetchL1HeadBlock,
  fetchProveTimes,
  fetchVerifyTimes,
  fetchL2BlockTimes,
  fetchL2GasUsed,
  fetchSequencerDistribution,
  fetchAllBlockTransactions,
  fetchBatchBlobCounts,
  fetchL2TxFee,
  fetchCloudCost,
  fetchAvgL2Tps,
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
  const [
    l2CadenceRes,
    batchCadenceRes,
    avgProveRes,
    avgVerifyRes,
    avgTpsRes,
    preconfRes,
    l2ReorgsRes,
    slashingCountRes,
    forcedInclusionCountRes,
    l2BlockRes,
    l1BlockRes,
    proveTimesRes,
    verifyTimesRes,
    l2TimesRes,
    l2GasUsedRes,
    sequencerDistRes,
    blockTxRes,
    batchBlobCountsRes,
    l2TxFeeRes,
    cloudCostRes,
  ] = await Promise.all([
    fetchL2BlockCadence(
      timeRange,
      selectedSequencer ? getSequencerAddress(selectedSequencer) : undefined,
    ),
    fetchBatchPostingCadence(timeRange),
    fetchAvgProveTime(timeRange),
    fetchAvgVerifyTime(timeRange),
    fetchAvgL2Tps(
      timeRange,
      selectedSequencer ? getSequencerAddress(selectedSequencer) : undefined,
    ),
    fetchPreconfData(),
    fetchL2Reorgs(timeRange),
    fetchSlashingEventCount(timeRange),
    fetchForcedInclusionCount(timeRange),
    fetchL2HeadBlock(timeRange),
    fetchL1HeadBlock(timeRange),
    fetchProveTimes(timeRange),
    fetchVerifyTimes(timeRange),
    fetchL2BlockTimes(
      timeRange,
      selectedSequencer ? getSequencerAddress(selectedSequencer) : undefined,
    ),
    fetchL2GasUsed(
      timeRange,
      selectedSequencer ? getSequencerAddress(selectedSequencer) : undefined,
    ),
    fetchSequencerDistribution(timeRange),
    fetchAllBlockTransactions(
      timeRange,
      selectedSequencer ? getSequencerAddress(selectedSequencer) : undefined,
    ),
    fetchBatchBlobCounts(timeRange),
    fetchL2TxFee(
      timeRange,
      selectedSequencer ? getSequencerAddress(selectedSequencer) : undefined,
    ),
    fetchCloudCost(timeRange),
  ]);

  const allResults = [
    l2CadenceRes,
    batchCadenceRes,
    avgProveRes,
    avgVerifyRes,
    avgTpsRes,
    preconfRes,
    l2ReorgsRes,
    slashingCountRes,
    forcedInclusionCountRes,
    l2BlockRes,
    l1BlockRes,
    proveTimesRes,
    verifyTimesRes,
    l2TimesRes,
    l2GasUsedRes,
    sequencerDistRes,
    blockTxRes,
    batchBlobCountsRes,
  ];

  return {
    l2Cadence: l2CadenceRes.data,
    batchCadence: batchCadenceRes.data,
    avgProve: avgProveRes.data,
    avgVerify: avgVerifyRes.data,
    avgTps: avgTpsRes.data,
    preconfData: preconfRes.data,
    l2Reorgs: l2ReorgsRes.data,
    slashings: slashingCountRes.data,
    forcedInclusions: forcedInclusionCountRes.data,
    l2Block: l2BlockRes.data,
    l1Block: l1BlockRes.data,
    proveTimes: proveTimesRes.data || [],
    verifyTimes: verifyTimesRes.data || [],
    l2Times: l2TimesRes.data || [],
    l2Gas: l2GasUsedRes.data || [],
    sequencerDist: sequencerDistRes.data || [],
    txPerBlock: blockTxRes.data || [],
    blobsPerBatch: batchBlobCountsRes.data || [],
    l2TxFee: l2TxFeeRes.data,
    cloudCost: cloudCostRes.data,
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
