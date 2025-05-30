import React from 'react';
import { TimeRange } from '../types';
import { TimeRangeSelector, RefreshRateInput } from './DashboardHeader';

interface Column {
  key: string;
  label: string;
}

interface ExtraTable {
  title: string;
  columns: Column[];
  rows: Array<Record<string, string | number>>;
  onRowClick?: (row: Record<string, string | number>) => void;
  pagination?: {
    page: number;
    onNext: () => void;
    onPrev: () => void;
    disableNext?: boolean;
    disablePrev?: boolean;
  };
}

interface DataTableProps {
  title: string;
  columns: Column[];
  rows: Array<Record<string, string | number>>;
  onBack: () => void;
  onRowClick?: (row: Record<string, string | number>) => void;
  extraAction?: { label: string; onClick: () => void };
  extraTable?: ExtraTable;
  timeRange?: TimeRange;
  onTimeRangeChange?: (range: TimeRange) => void;
  refreshRate?: number;
  onRefreshRateChange?: (rate: number) => void;
  chart?: React.ReactNode;
}

export const DataTable: React.FC<DataTableProps> = ({
  title,
  columns,
  rows,
  onBack,
  onRowClick,
  extraAction,
  extraTable,
  timeRange,
  onTimeRangeChange,
  refreshRate,
  onRefreshRateChange,
  chart,
}) => {
  const ROWS_PER_PAGE = 50;
  const [page, setPage] = React.useState(0);

  React.useEffect(() => {
    setPage(0);
  }, [rows]);

  const pageRows = React.useMemo(
    () => rows.slice(page * ROWS_PER_PAGE, (page + 1) * ROWS_PER_PAGE),
    [rows, page],
  );

  const disablePrev = page === 0;
  const disableNext = (page + 1) * ROWS_PER_PAGE >= rows.length;

  return (
    <div className="p-4">
      <div className="flex items-center mb-4 space-x-4">
        <button
          onClick={onBack}
          className="text-[#e81899] flex items-center space-x-1"
        >
          <span>&larr;</span>
          <span>Back</span>
        </button>
        {extraAction && (
          <button onClick={extraAction.onClick} className="text-[#e81899]">
            {extraAction.label}
          </button>
        )}
        {timeRange && onTimeRangeChange && (
          <TimeRangeSelector
            currentTimeRange={timeRange}
            onTimeRangeChange={onTimeRangeChange}
          />
        )}
        {refreshRate !== undefined && onRefreshRateChange && (
          <RefreshRateInput
            refreshRate={refreshRate}
            onRefreshRateChange={onRefreshRateChange}
          />
        )}
      </div>
      <h2 className="text-xl font-semibold mb-2">{title}</h2>
      {chart && <div className="h-64 md:h-80 w-full mb-4">{chart}</div>}
      <div className="overflow-x-auto">
        <table className="min-w-full border divide-y divide-gray-200">
          <thead>
            <tr>
              {columns.map((col) => (
                <th key={col.key} className="px-2 py-1 text-left">
                  {col.label}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {pageRows.map((row, idx) => (
              <tr
                key={idx}
                className="border-t hover:bg-gray-50 cursor-pointer"
                onClick={onRowClick ? () => onRowClick(row) : undefined}
              >
                {columns.map((col) => (
                  <td key={col.key} className="px-2 py-1">
                    {row[col.key] as React.ReactNode}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="flex items-center justify-between mt-2">
        <button
          onClick={() => setPage((p) => p - 1)}
          disabled={disablePrev}
          className="text-[#e81899] disabled:text-gray-400"
        >
          Prev
        </button>
        <span>Page {page + 1}</span>
        <button
          onClick={() => setPage((p) => p + 1)}
          disabled={disableNext}
          className="text-[#e81899] disabled:text-gray-400"
        >
          Next
        </button>
      </div>

      {extraTable ? (
        <div className="mt-8">
          <h3 className="text-lg font-semibold mb-2">{extraTable.title}</h3>
          <div className="overflow-x-auto">
            <table className="min-w-full border divide-y divide-gray-200">
              <thead>
                <tr>
                  {extraTable.columns.map((col) => (
                    <th key={col.key} className="px-2 py-1 text-left">
                      {col.label}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {extraTable.rows.map((row, idx) => (
                  <tr
                    key={idx}
                    className="border-t hover:bg-gray-50 cursor-pointer"
                    onClick={
                      extraTable.onRowClick
                        ? () => extraTable.onRowClick!(row)
                        : undefined
                    }
                  >
                    {extraTable.columns.map((col) => (
                      <td key={col.key} className="px-2 py-1">
                        {row[col.key] as React.ReactNode}
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {extraTable.pagination && (
            <div className="flex items-center justify-between mt-2">
              <button
                onClick={extraTable.pagination.onPrev}
                disabled={extraTable.pagination.disablePrev}
                className="text-[#e81899] disabled:text-gray-400"
              >
                Prev
              </button>
              <span>Page {extraTable.pagination.page + 1}</span>
              <button
                onClick={extraTable.pagination.onNext}
                disabled={extraTable.pagination.disableNext}
                className="text-[#e81899] disabled:text-gray-400"
              >
                Next
              </button>
            </div>
          )}
        </div>
      ) : null}
    </div>
  );
};
