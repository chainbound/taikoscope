import { useState, useCallback, useMemo } from 'react';
import { MetricData, TimeRange } from '../types';
import { createMetrics, type MetricInputData } from '../utils/metricsCreator';
import { hasBadRequest, getErrorMessage } from '../utils/errorHandler';
import { fetchMainDashboardData, fetchEconomicsData } from '../utils/dataFetcher';
import { useSearchParams } from './useSearchParams';

export const useMetricsData = () => {
    const [metrics, setMetrics] = useState<MetricData[]>([]);
    const [loadingMetrics, setLoadingMetrics] = useState(true);
    const [errorMessage, setErrorMessage] = useState<string>('');

    const searchParams = useSearchParams();

    // Memoize the specific value we need to prevent infinite re-renders
    const viewParam = searchParams.get('view');
    const isEconomicsView = useMemo(() => viewParam === 'economics', [viewParam]);

    const fetchMetricsData = useCallback(
        async (timeRange: TimeRange, selectedSequencer: string | null) => {
            setLoadingMetrics(true);

            try {
                if (isEconomicsView) {
                    const data = await fetchEconomicsData(timeRange, selectedSequencer);

                    const anyBadRequest = hasBadRequest(data.badRequestResults);

                    const metricsInput: MetricInputData = {
                        avgTps: null,
                        l2Cadence: null,
                        batchCadence: null,
                        avgProve: null,
                        avgVerify: null,
                        activeGateways: null,
                        currentOperator: null,
                        nextOperator: null,
                        l2Reorgs: null,
                        slashings: null,
                        forcedInclusions: null,
                        missedProposals: null,
                        l2TxFee: data.l2TxFee,
                        l2Block: data.l2Block,
                        l1Block: data.l1Block,
                        cloudCost: null,
                    };

                    const currentMetrics = createMetrics(metricsInput);
                    setMetrics(currentMetrics);
                    setErrorMessage(getErrorMessage(anyBadRequest));
                } else {
                    const data = await fetchMainDashboardData(timeRange, selectedSequencer);

                    const anyBadRequest = hasBadRequest(data.badRequestResults);

                    const activeGateways = data.preconfData ? data.preconfData.candidates.length : null;
                    const currentOperator = data.preconfData?.current_operator ?? null;
                    const nextOperator = data.preconfData?.next_operator ?? null;

                    const metricsInput: MetricInputData = {
                        avgTps: data.avgTps,
                        l2Cadence: data.l2Cadence,
                        batchCadence: data.batchCadence,
                        avgProve: data.avgProve,
                        avgVerify: data.avgVerify,
                        activeGateways,
                        currentOperator,
                        nextOperator,
                        l2Reorgs: data.l2Reorgs,
                        slashings: data.slashings,
                        forcedInclusions: data.forcedInclusions,
                        missedProposals: data.missedProposals,
                        l2TxFee: data.l2TxFee,
                        cloudCost: data.cloudCost,
                        l2Block: data.l2Block,
                        l1Block: data.l1Block,
                    };

                    const currentMetrics = createMetrics(metricsInput);
                    setMetrics(currentMetrics);
                    setErrorMessage(getErrorMessage(anyBadRequest));

                    return {
                        chartData: {
                            proveTimes: data.proveTimes,
                            verifyTimes: data.verifyTimes,
                            l2Times: data.l2Times,
                            l2Gas: data.l2Gas,
                            l1Times: data.l1Times,
                            txPerBlock: data.txPerBlock,
                            blobsPerBatch: data.blobsPerBatch,
                            sequencerDist: data.sequencerDist,
                        }
                    };
                }
            } catch (error) {
                console.error('Failed to fetch metrics data:', error);
                setErrorMessage('Failed to fetch dashboard data. Please try again.');
            } finally {
                setLoadingMetrics(false);
            }
        },
        [isEconomicsView],
    );

    return useMemo(
        () => ({
            metrics,
            setMetrics,
            loadingMetrics,
            errorMessage,
            setErrorMessage,
            fetchMetricsData,
        }),
        [metrics, loadingMetrics, errorMessage, fetchMetricsData],
    );
};