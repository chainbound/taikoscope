import { useMetricsData } from './useMetricsData';
import { useChartsData } from './useChartsData';
import { useBlockData } from './useBlockData';
import { useRefreshTimer } from './useRefreshTimer';
import { useTableRouter } from './useTableRouter';
import { useNavigationHandler } from './useNavigationHandler';
import { useDataFetcher } from './useDataFetcher';
import { useSequencerHandler } from './useSequencerHandler';
import { useTableActions } from './useTableActions';
import { useTimeRangeSync } from './useTimeRangeSync';
import { useSearchParams } from 'react-router-dom';

export const useDashboardController = () => {
    const { timeRange, setTimeRange } = useTimeRangeSync();
    const [searchParams] = useSearchParams();

    // Data management hooks
    const metricsData = useMetricsData();
    const chartsData = useChartsData();
    const blockData = useBlockData();
    const refreshTimer = useRefreshTimer();

    // Sequencer handling
    const { selectedSequencer, setSelectedSequencer, sequencerList } = useSequencerHandler({
        chartsData,
        blockData,
        metricsData,
    });

    // Table actions
    const {
        tableView,
        tableLoading,
        setTableView,
        setTableLoading,
        openGenericTable,
        openTpsTable,
        openSequencerDistributionTable,
    } = useTableActions(
        timeRange,
        setTimeRange,
        selectedSequencer,
        chartsData.blockTxData,
        chartsData.l2BlockTimeData,
    );

    // Data fetching coordination
    const { handleManualRefresh } = useDataFetcher({
        timeRange,
        selectedSequencer,
        tableView,
        fetchMetricsData: metricsData.fetchMetricsData,
        updateChartsData: chartsData.updateChartsData,
        refreshRate: refreshTimer.refreshRate,
        updateLastRefresh: refreshTimer.updateLastRefresh,
    });

    // Navigation handling
    const { handleBack, handleSequencerChange } = useNavigationHandler({
        onError: metricsData.setErrorMessage,
    });

    // Table routing
    useTableRouter({
        timeRange,
        setTableView,
        setTableLoading,
        tableView,
        openGenericTable,
        openTpsTable,
        openSequencerDistributionTable,
        onError: metricsData.setErrorMessage,
    });

    // Combined sequencer change handler
    const handleSequencerChangeWithState = (seq: string | null) => {
        setSelectedSequencer(seq);
        handleSequencerChange(seq);
    };

    return {
        // State
        timeRange,
        setTimeRange,
        selectedSequencer,
        sequencerList,

        // Data
        metricsData,
        chartsData,
        blockData,
        refreshTimer,
        searchParams,

        // Table state
        tableView,
        tableLoading,

        // Handlers
        handleSequencerChangeWithState,
        handleBack,
        handleManualRefresh,

        // Table actions
        openGenericTable,
        openTpsTable,
        openSequencerDistributionTable,
    };
};