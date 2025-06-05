import { useCallback, useEffect, useMemo } from 'react';
import { TimeRange } from '../types';
import { useSearchParams } from 'react-router-dom';
import { TableViewState } from './useTableActions';

interface UseTableRouterProps {
    timeRange: TimeRange;
    setTableView: (view: TableViewState | null) => void;
    setTableLoading: (loading: boolean) => void;
    tableView: TableViewState | null;
    openGenericTable: (table: string, range?: TimeRange, params?: Record<string, any>) => Promise<void>;
    openTpsTable: () => void;
    openSequencerDistributionTable: (range: TimeRange, page: number, start?: number, end?: number) => Promise<void>;
    onError: (message: string) => void;
}

export const useTableRouter = ({
    timeRange,
    setTableView,
    setTableLoading,
    tableView,
    openGenericTable,
    openTpsTable,
    openSequencerDistributionTable,
    onError,
}: UseTableRouterProps) => {
    const [searchParams] = useSearchParams();

    // Extract specific search param values to avoid unstable object dependencies
    const urlParams = useMemo(() => ({
        view: searchParams.get('view'),
        table: searchParams.get('table'),
        range: searchParams.get('range') as TimeRange,
        address: searchParams.get('address'),
        page: searchParams.get('page'),
        start: searchParams.get('start'),
        end: searchParams.get('end'),
    }), [
        searchParams.get('view'),
        searchParams.get('table'),
        searchParams.get('range'),
        searchParams.get('address'),
        searchParams.get('page'),
        searchParams.get('start'),
        searchParams.get('end'),
    ]);

    const handleRouteChange = useCallback(() => {
        try {
            const params = urlParams;
            if (params.view !== 'table') {
                setTableView(null);
                return;
            }

            // If we already have a table view and it matches the current URL state, don't reload
            const table = params.table;
            const range = params.range || timeRange;

            if (tableView && tableView.timeRange === range) {
                return;
            }

            setTableLoading(true);

            // Add error boundary for table operations
            const handleTableError = (tableName: string, error: any) => {
                console.error(`Failed to open ${tableName} table:`, error);
                setTableLoading(false);
                onError(`Failed to load ${tableName} table. Please try again.`);
            };

            switch (table) {
                case 'sequencer-blocks': {
                    const addr = params.address;
                    if (addr) {
                        openGenericTable('sequencer-blocks', range, { address: addr })
                            .catch((err) => handleTableError('sequencer-blocks', err));
                    } else {
                        setTableLoading(false);
                    }
                    break;
                }
                case 'tps':
                    try {
                        openTpsTable();
                    } catch (err) {
                        handleTableError('TPS', err);
                    }
                    break;
                case 'sequencer-dist': {
                    const pageStr = params.page ?? '0';
                    const page = parseInt(pageStr, 10);
                    if (isNaN(page) || page < 0) {
                        console.warn('Invalid page parameter:', pageStr);
                        setTableLoading(false);
                        break;
                    }
                    const start = params.start;
                    const end = params.end;
                    openSequencerDistributionTable(
                        range,
                        page,
                        start ? Number(start) : undefined,
                        end ? Number(end) : undefined,
                    ).catch((err) => handleTableError('sequencer-distribution', err));
                    break;
                }
                default: {
                    if (table) {
                        openGenericTable(table, range)
                            .catch((err) => handleTableError(table, err));
                    } else {
                        setTableLoading(false);
                    }
                    break;
                }
            }
        } catch (err) {
            console.error('Failed to handle route change:', err);
            setTableLoading(false);
            onError('Navigation error occurred. Please try again.');
        }
    }, [
        urlParams,
        openGenericTable,
        openTpsTable,
        openSequencerDistributionTable,
        setTableView,
        setTableLoading,
        timeRange,
        tableView,
        onError,
    ]);

    // Handle route changes
    useEffect(() => {
        try {
            handleRouteChange();
        } catch (err) {
            console.error('Route change effect error:', err);
            onError('Navigation error occurred.');
        }
    }, [handleRouteChange]);

    return {
        handleRouteChange,
    };
};