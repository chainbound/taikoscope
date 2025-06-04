import React from 'react';
import { TimeRange } from '../types';
import {
  TimeRangeSelector,
  RefreshRateInput,
  SequencerSelector,
} from './DashboardHeader';
import { RefreshCountdown } from './RefreshCountdown';
import { TAIKO_PINK } from '../theme';

const DEFAULT_ROWS_PER_PAGE = 50;

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
  description?: React.ReactNode;
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
  lastRefresh?: number;
  onManualRefresh?: () => void;
  sequencers?: string[];
  selectedSequencer?: string | null;
  onSequencerChange?: (seq: string | null) => void;
  chart?: React.ReactNode;
  rowsPerPage?: number;
  isNavigating?: boolean;
}

export const DataTable: React.FC<DataTableProps> = ({
  title,
  description,
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
  lastRefresh,
  onManualRefresh,
  sequencers,
  selectedSequencer,
  onSequencerChange,
  chart,
  rowsPerPage = DEFAULT_ROWS_PER_PAGE,
  isNavigating = false,
}) => {
  const [page, setPage] = React.useState(0);

  React.useEffect(() => {
    setPage(0);
  }, [rows]);

  React.useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (isNavigating) return;
      
      try {
        if (event.key === 'Escape') {
          onBack();
        } else if (event.altKey && event.key === 'ArrowLeft') {
          event.preventDefault();
          onBack();
        }
      } catch (err) {
        console.error('Failed to handle keyboard navigation:', err);
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [onBack, isNavigating]);

  const pageRows = React.useMemo(
    () => rows.slice(page * rowsPerPage, (page + 1) * rowsPerPage),
    [rows, page, rowsPerPage],
  );

  const disablePrev = page === 0;
  const disableNext = (page + 1) * rowsPerPage >= rows.length;

  return (
    <div className="p-4">
      <div className="flex items-center mb-4 space-x-4">
        <button
          onClick={() => {
            try {
              onBack();
            } catch (err) {
              console.error('Failed to navigate back:', err);
            }
          }}
          disabled={isNavigating}
          className={`flex items-center space-x-1 ${isNavigating ? 'opacity-50 cursor-not-allowed' : ''}`}
          style={{ color: TAIKO_PINK }}
        >
          <span>&larr;</span>
          <span>Back</span>
        </button>
        {extraAction && (
          <button
            onClick={extraAction.onClick}
            className=""
            style={{ color: TAIKO_PINK }}
          >
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
        {refreshRate !== undefined &&
          lastRefresh !== undefined &&
          onManualRefresh && (
            <RefreshCountdown
              refreshRate={refreshRate}
              lastRefresh={lastRefresh}
              onRefresh={onManualRefresh}
            />
          )}
        {sequencers && onSequencerChange && (
          <SequencerSelector
            sequencers={sequencers}
            value={selectedSequencer ?? null}
            onChange={onSequencerChange}
          />
        )}
      </div>
      <h2 className="text-xl font-semibold mb-2">{title}</h2>
      {description && <p className="text-gray-600 mb-2">{description}</p>}
      {chart && (
        <div className="h-64 md:h-80 w-full mb-4">
          <React.Suspense
            fallback={
              <div className="flex items-center justify-center h-full text-gray-500">
                Loading...
              </div>
            }
          >
            {chart}
          </React.Suspense>
        </div>
      )}
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
            {pageRows.length === 0 ? (
              <tr>
                <td
                  colSpan={columns.length}
                  className="px-2 py-4 text-center text-gray-500"
                >
                  No data available
                </td>
              </tr>
            ) : (
              pageRows.map((row, idx) => (
                <tr
                  key={idx}
                  className={`border-t hover:bg-gray-50 ${onRowClick && !isNavigating ? 'cursor-pointer' : ''} ${isNavigating ? 'pointer-events-none opacity-50' : ''}`}
                  onClick={onRowClick && !isNavigating ? () => {
                    try {
                      onRowClick(row);
                    } catch (err) {
                      console.error('Failed to handle row click:', err);
                    }
                  } : undefined}
                >
                  {columns.map((col) => (
                    <td key={col.key} className="px-2 py-1">
                      {row[col.key] as React.ReactNode}
                    </td>
                  ))}
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
      {rows.length > 0 && (
        <div className="flex items-center justify-between mt-2">
          <button
            onClick={() => setPage((p) => p - 1)}
            disabled={disablePrev || isNavigating}
            className={`disabled:text-gray-400 ${isNavigating ? 'opacity-50' : ''}`}
            style={disablePrev || isNavigating ? undefined : { color: TAIKO_PINK }}
          >
            Prev
          </button>
          <span>Page {page + 1}</span>
          <button
            onClick={() => setPage((p) => p + 1)}
            disabled={disableNext || isNavigating}
            className={`disabled:text-gray-400 ${isNavigating ? 'opacity-50' : ''}`}
            style={disableNext || isNavigating ? undefined : { color: TAIKO_PINK }}
          >
            Next
          </button>
        </div>
      )}

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
                {extraTable.rows.length === 0 ? (
                  <tr>
                    <td
                      colSpan={extraTable.columns.length}
                      className="px-2 py-4 text-center text-gray-500"
                    >
                      No data available
                    </td>
                  </tr>
                ) : (
                  extraTable.rows.map((row, idx) => (
                    <tr
                      key={idx}
                      className={`border-t hover:bg-gray-50 ${extraTable.onRowClick && !isNavigating ? 'cursor-pointer' : ''} ${isNavigating ? 'pointer-events-none opacity-50' : ''}`}
                      onClick={
                        extraTable.onRowClick && !isNavigating
                          ? () => {
                            try {
                              extraTable.onRowClick!(row);
                            } catch (err) {
                              console.error('Failed to handle extra table row click:', err);
                            }
                          }
                          : undefined
                      }
                    >
                      {extraTable.columns.map((col) => (
                        <td key={col.key} className="px-2 py-1">
                          {row[col.key] as React.ReactNode}
                        </td>
                      ))}
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
          {extraTable.pagination && extraTable.rows.length > 0 && (
            <div className="flex items-center justify-between mt-2">
              <button
                onClick={extraTable.pagination.onPrev}
                disabled={extraTable.pagination.disablePrev || isNavigating}
                className={`disabled:text-gray-400 ${isNavigating ? 'opacity-50' : ''}`}
                style={
                  extraTable.pagination.disablePrev || isNavigating
                    ? undefined
                    : { color: TAIKO_PINK }
                }
              >
                Prev
              </button>
              <span>Page {extraTable.pagination.page + 1}</span>
              <button
                onClick={extraTable.pagination.onNext}
                disabled={extraTable.pagination.disableNext || isNavigating}
                className={`disabled:text-gray-400 ${isNavigating ? 'opacity-50' : ''}`}
                style={
                  extraTable.pagination.disableNext || isNavigating
                    ? undefined
                    : { color: TAIKO_PINK }
                }
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
