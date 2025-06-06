import React from 'react';
import { DataTable } from '../DataTable';
import { LoadingState } from '../layout/LoadingState';
import { TableViewState } from '../../hooks/useTableActions';

interface TableViewProps {
  tableView?: TableViewState;
  tableLoading: boolean;
  isNavigating: boolean;
  onBack: () => void;
}

export const TableView: React.FC<TableViewProps> = ({
  tableView,
  tableLoading,
  isNavigating,
  onBack,
}) => {
  if (tableLoading || isNavigating) {
    return (
      <LoadingState message={isNavigating ? 'Navigating...' : 'Loading...'} />
    );
  }

  if (!tableView) {
    return <div>No table data available</div>;
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
      chart={tableView.chart}
      isNavigating={isNavigating}
      allRows={tableView.allRows}
      useClientSidePagination={tableView.useClientSidePagination}
      totalRecords={tableView.totalRecords}
      timeRange={tableView.timeRange}
    />
  );
};
