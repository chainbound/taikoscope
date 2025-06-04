import { useCallback, useEffect, useMemo, useRef } from 'react';
import { TimeRange } from '../types';
import { useSearchParams } from './useSearchParams';
import { TableViewState } from './useTableActions';

interface UseDataFetcherProps {
    timeRange: TimeRange;
    selectedSequencer: string | null;
    tableView: TableViewState | null;
    metricsData: {
        fetchMetricsData: (timeRange: TimeRange, selectedSequencer: string | null) => Promise<any>;
    };
    chartsData: {
        updateChartsData: (data: any) => void;
    };
    refreshTimer: {
        refreshRate: number;
        updateLastRefresh: () => void;
    };
}

export const useDataFetcher = ({
    timeRange,
    selectedSequencer,
    tableView,
    metricsData,
    chartsData,
    refreshTimer,
}: UseDataFetcherProps) => {
    const searchParams = useSearchParams();
    
    // Memoize the specific value we need to prevent infinite re-renders
    const viewParam = searchParams.get('view');
    const isTableView = useMemo(() => 
        tableView || viewParam === 'table', 
        [tableView, viewParam]
    );

    // Prevent duplicate requests
    const fetchInProgressRef = useRef(false);
    
    const fetchData = useCallback(async () => {
        // Prevent duplicate concurrent requests
        if (fetchInProgressRef.current) {
            console.log('Fetch already in progress, skipping duplicate request');
            return;
        }
        
        fetchInProgressRef.current = true;
        
        try {
            refreshTimer.updateLastRefresh();

            const result = await metricsData.fetchMetricsData(timeRange, selectedSequencer);

            // Update charts data if available (main dashboard view)
            if (result?.chartData) {
                chartsData.updateChartsData(result.chartData);
            }
        } catch (error) {
            console.error('Data fetch failed:', error);
        } finally {
            fetchInProgressRef.current = false;
        }
    }, [timeRange, selectedSequencer, metricsData, chartsData, refreshTimer]);

    const handleManualRefresh = useCallback(() => {
        if (tableView && tableView.onRefresh) {
            // If we're in a table view and it has a refresh function, use that
            tableView.onRefresh();
        } else {
            // Otherwise refresh the main dashboard data
            void fetchData();
        }
    }, [fetchData, tableView?.onRefresh]);

    // Auto-refresh effect
    useEffect(() => {
        if (isTableView) return;
        fetchData();
        const interval = setInterval(fetchData, Math.max(refreshTimer.refreshRate, 60000));
        return () => clearInterval(interval);
    }, [timeRange, fetchData, refreshTimer.refreshRate, isTableView]);

    return {
        fetchData,
        handleManualRefresh,
    };
};