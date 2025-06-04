import React, { useCallback } from 'react';
import { DashboardHeader } from '../DashboardHeader';
import { ErrorDisplay } from '../layout/ErrorDisplay';
import { MetricsGrid } from '../layout/MetricsGrid';
import { ChartsGrid } from '../layout/ChartsGrid';
import { TAIKO_PINK } from '../../theme';
import { TimeRange, MetricData } from '../../types';
import { useSearchParams } from '../../hooks/useSearchParams';

interface DashboardViewProps {
    timeRange: TimeRange;
    onTimeRangeChange: (range: TimeRange) => void;
    selectedSequencer: string | null;
    onSequencerChange: (seq: string | null) => void;
    sequencerList: string[];

    // Data hooks
    metricsData: {
        metrics: MetricData[];
        loadingMetrics: boolean;
        errorMessage: string;
        setErrorMessage: (msg: string) => void;
    };
    chartsData: any;
    blockData: {
        l2HeadBlock: string;
        l1HeadBlock: string;
    };
    refreshTimer: {
        refreshRate: number;
        setRefreshRate: (rate: number) => void;
        lastRefresh: number;
    };

    // Actions
    onManualRefresh: () => void;
    onOpenTable: (table: string, timeRange?: TimeRange) => void;
    onOpenTpsTable: () => void;
    onOpenSequencerDistributionTable: (timeRange: TimeRange, page: number) => void;
}

export const DashboardView: React.FC<DashboardViewProps> = ({
    timeRange,
    onTimeRangeChange,
    selectedSequencer,
    onSequencerChange,
    sequencerList,
    metricsData,
    chartsData,
    blockData,
    refreshTimer,
    onManualRefresh,
    onOpenTable,
    onOpenTpsTable,
    onOpenSequencerDistributionTable,
}) => {
    const searchParams = useSearchParams();
    const isEconomicsView = searchParams.get('view') === 'economics';

    const visibleMetrics = React.useMemo(
        () =>
            metricsData.metrics.filter((m) => {
                if (selectedSequencer && m.group === 'Sequencers') return false;
                if (isEconomicsView) return m.group === 'Network Economics';
                return m.group !== 'Network Economics';
            }),
        [metricsData.metrics, selectedSequencer, isEconomicsView],
    );

    const groupedMetrics = visibleMetrics.reduce<Record<string, MetricData[]>>(
        (acc, m) => {
            const group = m.group ?? 'Other';
            if (!acc[group]) acc[group] = [];
            acc[group].push(m);
            return acc;
        },
        {},
    );

    const groupOrder = isEconomicsView
        ? ['Network Economics']
        : ['Network Performance', 'Network Health', 'Sequencers', 'Other'];

    const skeletonGroupCounts: Record<string, number> = isEconomicsView
        ? { 'Network Economics': 1 }
        : {
            'Network Performance': 5,
            'Network Health': 4,
            Sequencers: 3,
        };

    const displayGroupName = useCallback(
        (group: string): string => {
            if (!selectedSequencer) return group;
            if (group === 'Network Performance') return 'Sequencer Performance';
            if (group === 'Network Health') return 'Sequencer Health';
            return group;
        },
        [selectedSequencer],
    );

    const displayedGroupOrder = selectedSequencer
        ? groupOrder.filter((g) => g !== 'Sequencers')
        : groupOrder;

    const handleResetNavigation = useCallback(() => {
        try {
            searchParams.resetNavigation();
            metricsData.setErrorMessage('');
        } catch (err) {
            console.error('Failed to reset navigation:', err);
        }
    }, [searchParams, metricsData]);

    const handleClearError = useCallback(() => {
        metricsData.setErrorMessage('');
    }, [metricsData]);

    const getMetricAction = useCallback((title: string) => {
        const actions: Record<string, () => void> = {
            'Avg. L2 TPS': onOpenTpsTable,
            'L2 Reorgs': () => onOpenTable('reorgs'),
            'Slashing Events': () => onOpenTable('slashings'),
            'Forced Inclusions': () => onOpenTable('forced-inclusions'),
            'Missed Proposals': () => onOpenTable('missed-proposals'),
            'Active Sequencers': () => onOpenTable('gateways'),
            'Batch Posting Cadence': () => onOpenTable('batch-posting-cadence'),
        };
        return actions[title];
    }, [onOpenTable, onOpenTpsTable]);

    return (
        <div
            className="min-h-screen bg-white text-gray-800 p-4 md:p-6 lg:p-8"
            style={{ fontFamily: "'Inter', sans-serif" }}
        >
            <DashboardHeader
                timeRange={timeRange}
                onTimeRangeChange={onTimeRangeChange}
                refreshRate={refreshTimer.refreshRate}
                onRefreshRateChange={refreshTimer.setRefreshRate}
                lastRefresh={refreshTimer.lastRefresh}
                onManualRefresh={onManualRefresh}
                sequencers={sequencerList}
                selectedSequencer={selectedSequencer}
                onSequencerChange={onSequencerChange}
            />

            <ErrorDisplay
                errorMessage={metricsData.errorMessage}
                navigationError={searchParams.navigationState.lastError}
                errorCount={searchParams.navigationState.errorCount}
                onResetNavigation={handleResetNavigation}
                onClearError={handleClearError}
            />

            <main className="mt-6">
                <MetricsGrid
                    isLoading={metricsData.loadingMetrics}
                    groupedMetrics={groupedMetrics}
                    groupOrder={displayedGroupOrder}
                    skeletonGroupCounts={skeletonGroupCounts}
                    displayGroupName={displayGroupName}
                    onMetricAction={getMetricAction}
                />

                {!isEconomicsView && (
                    <ChartsGrid
                        isLoading={metricsData.loadingMetrics}
                        timeRange={timeRange}
                        selectedSequencer={selectedSequencer}
                        chartsData={chartsData}
                        onOpenTable={onOpenTable}
                        onOpenSequencerDistributionTable={onOpenSequencerDistributionTable}
                    />
                )}
            </main>

            {/* Footer for Block Numbers */}
            <footer className="mt-8 pt-6 border-t border-gray-200">
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-center md:text-left">
                    <div>
                        <span className="text-sm text-gray-500">L2 Head Block</span>
                        <p className="text-2xl font-semibold" style={{ color: TAIKO_PINK }}>
                            {blockData.l2HeadBlock}
                        </p>
                    </div>
                    <div>
                        <span className="text-sm text-gray-500">L1 Head Block</span>
                        <p className="text-2xl font-semibold" style={{ color: TAIKO_PINK }}>
                            {blockData.l1HeadBlock}
                        </p>
                    </div>
                </div>
            </footer>
        </div>
    );
};
