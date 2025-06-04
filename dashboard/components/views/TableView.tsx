import React from 'react';
import { DataTable } from '../DataTable';
import { LoadingState } from '../layout/LoadingState';
import { TableViewState } from '../../hooks/useTableActions';

interface TableViewProps {
    tableView: TableViewState;
    tableLoading: boolean;
    isNavigating: boolean;
    refreshTimer: {
        refreshRate: number;
        setRefreshRate: (rate: number) => void;
        lastRefresh: number;
    };
    sequencerList: string[];
    selectedSequencer: string | null;
    onSequencerChange: (seq: string | null) => void;
    onBack: () => void;
    onManualRefresh: () => void;
}

export const TableView: React.FC<TableViewProps> = ({
    tableView,
    tableLoading,
    isNavigating,
    refreshTimer,
    sequencerList,
    selectedSequencer,
    onSequencerChange,
    onBack,
    onManualRefresh,
}) => {
    if (tableLoading || isNavigating) {
        return (
            <LoadingState
                message={isNavigating ? 'Navigating...' : 'Loading...'}
            />
        );
    }

    return (
        <DataTable
            title={tableView.title}
            description={tableView.description}
            columns={tableView.columns}
            rows={tableView.rows}
            onBack={onBack}
            onRowClick={tableView.onRowClick}
            extraAction={tableView.extraAction}
            extraTable={tableView.extraTable}
            timeRange={tableView.timeRange}
            onTimeRangeChange={tableView.onTimeRangeChange}
            refreshRate={refreshTimer.refreshRate}
            onRefreshRateChange={refreshTimer.setRefreshRate}
            lastRefresh={refreshTimer.lastRefresh}
            onManualRefresh={onManualRefresh}
            sequencers={sequencerList}
            selectedSequencer={selectedSequencer}
            onSequencerChange={onSequencerChange}
            chart={tableView.chart}
            isNavigating={isNavigating}
        />
    );
};