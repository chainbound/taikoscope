import React from 'react';

const DEFAULT_ROWS_PER_PAGE = 50;

interface Column {
  key: string;
  label: string;
  sortable?: boolean;
}

interface ExtraTable {
  title: string;
  columns: Column[];
  rows: Array<Record<string, React.ReactNode | string | number>>;
  onRowClick?: (row: Record<string, React.ReactNode | string | number>) => void;
  pagination?: {
    page: number;
    onNext: () => void;
    onPrev: () => void;
    disableNext?: boolean;
    disablePrev?: boolean;
  };
}

interface ServerPagination {
  page: number;
  onNext: () => void;
  onPrev: () => void;
  disableNext?: boolean;
  disablePrev?: boolean;
}

interface DataTableProps {
  title: string;
  description?: React.ReactNode;
  columns: Column[];
  rows: Array<Record<string, React.ReactNode | string | number>>;
  onBack: () => void;
  onRowClick?: (row: Record<string, React.ReactNode | string | number>) => void;
  extraAction?: { label: string; onClick: () => void };
  extraTable?: ExtraTable;
  chart?: React.ReactNode;
  rowsPerPage?: number;
  isNavigating?: boolean;
  allRows?: Array<Record<string, React.ReactNode | string | number>>;
  useClientSidePagination?: boolean;
  totalRecords?: number;
  timeRange?: string;
  defaultSortBy?: string;
  defaultSortDirection?: 'asc' | 'desc';
  serverPagination?: ServerPagination;
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
  chart,
  rowsPerPage = DEFAULT_ROWS_PER_PAGE,
  isNavigating = false,
  allRows,
  useClientSidePagination = false,
  totalRecords,
  timeRange,
  defaultSortBy,
  defaultSortDirection,
  serverPagination,
}) => {
  const [page, setPage] = React.useState(serverPagination?.page ?? 0);
  const [sortBy, setSortBy] = React.useState<string>(defaultSortBy ?? '');
  const [sortDirection, setSortDirection] = React.useState<'asc' | 'desc'>(
    defaultSortDirection ?? 'asc',
  );

  const prevRowsLength = React.useRef(rows.length);

  React.useEffect(() => {
    if (serverPagination) return;
    if (rows.length !== prevRowsLength.current) {
      setPage(0);
    }
    prevRowsLength.current = rows.length;
  }, [rows, serverPagination]);

  React.useEffect(() => {
    if (serverPagination) {
      setPage(serverPagination.page);
    }
  }, [serverPagination]);

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

  // Use client-side pagination if enabled and allRows are provided
  const dataSource = useClientSidePagination && allRows ? allRows : rows;
  const currentTotalRecords =
    useClientSidePagination && totalRecords ? totalRecords : dataSource.length;

  const sortedData = React.useMemo(() => {
    if (!sortBy) return dataSource;
    return [...dataSource].sort((a, b) => {
      const aVal = String(a[sortBy]);
      const bVal = String(b[sortBy]);
      const comparison = aVal.localeCompare(bVal, undefined, { numeric: true });
      return sortDirection === 'desc' ? -comparison : comparison;
    });
  }, [dataSource, sortBy, sortDirection]);

  const pageRows = React.useMemo(() => {
    const data = sortedData;
    if (serverPagination) {
      return data;
    }
    if (useClientSidePagination && allRows) {
      return data.slice(page * rowsPerPage, (page + 1) * rowsPerPage);
    }
    return data.slice(page * rowsPerPage, (page + 1) * rowsPerPage);
  }, [sortedData, page, rowsPerPage, useClientSidePagination, allRows, serverPagination]);

  const disablePrev = serverPagination
    ? serverPagination.disablePrev || false
    : page === 0;
  const disableNext = serverPagination
    ? serverPagination.disableNext || false
    : (page + 1) * rowsPerPage >= currentTotalRecords;

  return (
    <div className="p-4 bg-bg text-fg">
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
          style={{ color: 'var(--color-brand)' }}
        >
          <span>&larr;</span>
          <span>Back</span>
        </button>
        {extraAction && (
          <button
            onClick={extraAction.onClick}
            className=""
            style={{ color: 'var(--color-brand)' }}
          >
            {extraAction.label}
          </button>
        )}
      </div>
      <h2 className="text-xl font-semibold mb-2 text-gray-900 dark:text-gray-100">
        {title}
      </h2>
      {description && (
        <p className="text-gray-600 dark:text-gray-400 mb-2">{description}</p>
      )}

      {/* Data scope indicator */}
      {useClientSidePagination && totalRecords && (
        <div className="mb-2 p-2 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-700 rounded text-sm text-blue-700 dark:text-blue-300">
          📊 Showing {pageRows.length} of {totalRecords} records from{' '}
          {timeRange || 'selected time range'}
          {chart && ' (Chart displays all records)'}
        </div>
      )}

      {chart && (
        <div className="h-64 md:h-80 w-full mb-4">
          <React.Suspense
            fallback={
              <div className="flex items-center justify-center h-full text-gray-500 dark:text-gray-400">
                Loading...
              </div>
            }
          >
            {chart}
          </React.Suspense>
        </div>
      )}
      <div className="overflow-x-auto">
        <table className="min-w-full border border-border divide-y divide-gray-200 dark:divide-gray-700 bg-card text-card-fg">
          <thead>
            <tr>
              {columns.map((col) => (
                <th
                  key={col.key}
                  className={`px-2 py-1 text-left ${
                    col.sortable ? 'cursor-pointer select-none' : ''
                  }`}
                  onClick={
                    col.sortable
                      ? () => {
                          if (sortBy === col.key) {
                            setSortDirection((d) => (d === 'asc' ? 'desc' : 'asc'));
                          } else {
                            setSortBy(col.key);
                            setSortDirection('desc');
                          }
                        }
                      : undefined
                  }
                >
                  <span>{col.label}</span>
                  {col.sortable && sortBy === col.key && (
                    <span className="ml-1">{sortDirection === 'asc' ? '↑' : '↓'}</span>
                  )}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {pageRows.length === 0 ? (
              <tr>
                <td
                  colSpan={columns.length}
                  className="px-2 py-4 text-center text-gray-500 dark:text-gray-400"
                >
                  No data available
                </td>
              </tr>
            ) : (
              pageRows.map((row, idx) => (
                <tr
                  key={idx}
                  className={`border-t border-border hover:bg-muted ${onRowClick && !isNavigating ? 'cursor-pointer' : ''} ${isNavigating ? 'pointer-events-none opacity-50' : ''}`}
                  onClick={
                    onRowClick && !isNavigating
                      ? () => {
                          try {
                            onRowClick(row);
                          } catch (err) {
                            console.error('Failed to handle row click:', err);
                          }
                        }
                      : undefined
                  }
                >
                  {columns.map((col) => (
                    <td
                      key={col.key}
                      className="px-2 py-1"
                    >
                      {row[col.key] as React.ReactNode}
                    </td>
                  ))}
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
      {serverPagination ? (
        <div className="flex items-center justify-between mt-2">
          <button
            onClick={serverPagination.onPrev}
            disabled={serverPagination.disablePrev || isNavigating}
            className={`disabled:text-gray-400 dark:disabled:text-gray-500 ${isNavigating ? 'opacity-50' : ''}`}
            style={serverPagination.disablePrev || isNavigating ? undefined : { color: 'var(--color-brand)' }}
          >
            Prev
          </button>
          <span className="text-gray-900 dark:text-gray-100">
            Page {serverPagination.page + 1}
          </span>
          <button
            onClick={serverPagination.onNext}
            disabled={serverPagination.disableNext || isNavigating}
            className={`disabled:text-gray-400 dark:disabled:text-gray-500 ${isNavigating ? 'opacity-50' : ''}`}
            style={serverPagination.disableNext || isNavigating ? undefined : { color: 'var(--color-brand)' }}
          >
            Next
          </button>
        </div>
      ) : (
        currentTotalRecords > 0 && (
          <div className="flex items-center justify-between mt-2">
            <button
              onClick={() => setPage((p) => p - 1)}
              disabled={disablePrev || isNavigating}
              className={`disabled:text-gray-400 dark:disabled:text-gray-500 ${isNavigating ? 'opacity-50' : ''}`}
              style={disablePrev || isNavigating ? undefined : { color: 'var(--color-brand)' }}
            >
              Prev
            </button>
            <span className="text-gray-900 dark:text-gray-100">
              Page {page + 1} of {Math.ceil(currentTotalRecords / rowsPerPage)}
            </span>
            <button
              onClick={() => setPage((p) => p + 1)}
              disabled={disableNext || isNavigating}
              className={`disabled:text-gray-400 dark:disabled:text-gray-500 ${isNavigating ? 'opacity-50' : ''}`}
              style={disableNext || isNavigating ? undefined : { color: 'var(--color-brand)' }}
            >
              Next
            </button>
          </div>
        )
      )}

      {extraTable ? (
        <div className="mt-8">
          <h3 className="text-lg font-semibold mb-2 text-gray-900 dark:text-gray-100">
            {extraTable.title}
          </h3>
          <div className="overflow-x-auto">
            <table className="min-w-full border border-gray-200 dark:border-gray-700 divide-y divide-gray-200 dark:divide-gray-700 bg-white dark:bg-gray-800">
              <thead>
                <tr>
                  {extraTable.columns.map((col) => (
                    <th
                      key={col.key}
                      className="px-2 py-1 text-left text-gray-900 dark:text-gray-100"
                    >
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
                      className="px-2 py-4 text-center text-gray-500 dark:text-gray-400"
                    >
                      No data available
                    </td>
                  </tr>
                ) : (
                  extraTable.rows.map((row, idx) => (
                    <tr
                      key={idx}
                      className={`border-t border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-700 ${extraTable.onRowClick && !isNavigating ? 'cursor-pointer' : ''} ${isNavigating ? 'pointer-events-none opacity-50' : ''}`}
                      onClick={
                        extraTable.onRowClick && !isNavigating
                          ? () => {
                              try {
                                extraTable.onRowClick!(row);
                              } catch (err) {
                                console.error(
                                  'Failed to handle extra table row click:',
                                  err,
                                );
                              }
                            }
                          : undefined
                      }
                    >
                      {extraTable.columns.map((col) => (
                        <td
                          key={col.key}
                          className="px-2 py-1 text-gray-900 dark:text-gray-100"
                        >
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
                className={`disabled:text-gray-400 dark:disabled:text-gray-500 ${isNavigating ? 'opacity-50' : ''}`}
                style={extraTable.pagination.disablePrev || isNavigating ? undefined : { color: 'var(--color-brand)' }}
              >
                Prev
              </button>
              <span className="text-gray-900 dark:text-gray-100">
                Page {extraTable.pagination.page + 1}
              </span>
              <button
                onClick={extraTable.pagination.onNext}
                disabled={extraTable.pagination.disableNext || isNavigating}
                className={`disabled:text-gray-400 dark:disabled:text-gray-500 ${isNavigating ? 'opacity-50' : ''}`}
                style={extraTable.pagination.disableNext || isNavigating ? undefined : { color: 'var(--color-brand)' }}
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
