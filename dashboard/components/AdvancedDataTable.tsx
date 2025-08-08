import React, { useState, useMemo, useCallback, useEffect } from 'react';
import VirtualizedTable from './VirtualizedTable';
import { Input } from './ui/Input';
import {
  exportTableData,
  ExportFormat,
  validateExportData,
} from '../utils/exportUtils';
import { tableCache } from '../utils/smartCache';
// brand color provided via Tailwind token

interface Column {
  key: string;
  label: string;
  width?: number;
  sortable?: boolean;
  filterable?: boolean;
}

interface ExtraTable {
  title: string;
  columns: Column[];
  rows: Array<Record<string, React.ReactNode | string | number>>;
  onRowClick?: (row: Record<string, React.ReactNode | string | number>) => void;
}

interface AdvancedDataTableProps {
  title: string;
  description?: React.ReactNode;
  columns: Column[];
  rows: Array<Record<string, React.ReactNode | string | number>>;
  onBack: () => void;
  onRowClick?: (row: Record<string, React.ReactNode | string | number>) => void;
  extraAction?: { label: string; onClick: () => void };
  extraTable?: ExtraTable;
  chart?: React.ReactNode;
  height?: number;
  rowHeight?: number;
  cacheKey?: string;
  timeRange?: string;
  // Advanced features
  enableSearch?: boolean;
  enableFilters?: boolean;
  enableSorting?: boolean;
  enableExport?: boolean;
  enableVirtualization?: boolean;
  searchPlaceholder?: string;
}

export const AdvancedDataTable: React.FC<AdvancedDataTableProps> = ({
  title,
  description,
  columns,
  rows,
  onBack,
  onRowClick,
  extraAction,
  extraTable,
  chart,
  height = 600,
  rowHeight = 40,
  cacheKey,
  timeRange,
  enableSearch = true,
  enableFilters = true,
  enableSorting = true,
  enableExport = true,
  enableVirtualization = true,
  searchPlaceholder = 'Search table data...',
}) => {
  const [searchTerm, setSearchTerm] = useState('');
  const [filters, setFilters] = useState<Record<string, string>>({});
  const [sortBy, setSortBy] = useState<string>('');
  const [sortDirection, setSortDirection] = useState<'asc' | 'desc'>('asc');
  const [isExporting, setIsExporting] = useState(false);
  const [exportProgress, setExportProgress] = useState(0);

  // Cache table state
  const stateKey = `${cacheKey || title}_state`;

  useEffect(() => {
    if (cacheKey) {
      const cachedState = tableCache.get(stateKey);
      if (cachedState) {
        setSearchTerm(cachedState.searchTerm || '');
        setFilters(cachedState.filters || {});
        setSortBy(cachedState.sortBy || '');
        setSortDirection(cachedState.sortDirection || 'asc');
      }
    }
  }, [cacheKey, stateKey]);

  // Save state to cache when it changes
  useEffect(() => {
    if (cacheKey) {
      tableCache.set(
        stateKey,
        {
          searchTerm,
          filters,
          sortBy,
          sortDirection,
        },
        undefined,
        30 * 60 * 1000,
      ); // 30 minutes TTL
    }
  }, [searchTerm, filters, sortBy, sortDirection, cacheKey, stateKey]);

  // Configure columns for advanced features
  const enhancedColumns = useMemo(() => {
    return columns.map((col) => ({
      ...col,
      sortable: enableSorting && col.sortable !== false,
      filterable: enableFilters && col.filterable !== false,
    }));
  }, [columns, enableSorting, enableFilters]);

  // Handle sorting
  const handleSort = useCallback(
    (column: string, direction: 'asc' | 'desc') => {
      setSortBy(column);
      setSortDirection(direction);
    },
    [],
  );

  // Handle filtering
  const handleFilter = useCallback((newFilters: Record<string, string>) => {
    setFilters(newFilters);
  }, []);

  // Handle search
  const handleSearch = useCallback((term: string) => {
    setSearchTerm(term);
  }, []);

  // Export functionality
  const handleExport = useCallback(
    async (format: ExportFormat) => {
      const validation = validateExportData(rows, enhancedColumns);
      if (!validation.isValid) {
        alert(`Export failed: ${validation.errors.join(', ')}`);
        return;
      }

      setIsExporting(true);
      setExportProgress(0);

      try {
        const filename = `${title.toLowerCase().replace(/\s+/g, '-')}${timeRange ? `_${timeRange}` : ''}`;

        await exportTableData(rows, enhancedColumns, format, {
          filename,
          includeHeaders: true,
        });

        setExportProgress(100);

        // Show success message briefly
        setTimeout(() => {
          setIsExporting(false);
          setExportProgress(0);
        }, 1000);
      } catch (error) {
        console.error('Export failed:', error);
        alert(
          `Export failed: ${error instanceof Error ? error.message : 'Unknown error'}`,
        );
        setIsExporting(false);
        setExportProgress(0);
      }
    },
    [rows, enhancedColumns, title, timeRange],
  );

  // Clear all filters and search
  const handleClearAll = useCallback(() => {
    setSearchTerm('');
    setFilters({});
    setSortBy('');
    setSortDirection('asc');
  }, []);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onBack();
      } else if (event.ctrlKey || event.metaKey) {
        switch (event.key) {
          case 'f': {
            event.preventDefault();
            // Focus search input
            const searchInput = document.querySelector(
              'input[placeholder*="Search"]',
            ) as HTMLInputElement;
            searchInput?.focus();
            break;
          }
          case 'e':
            event.preventDefault();
            if (enableExport && !isExporting) {
              handleExport('csv');
            }
            break;
          case 'r':
            event.preventDefault();
            handleClearAll();
            break;
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onBack, enableExport, isExporting, handleExport, handleClearAll]);

  const hasActiveFilters =
    searchTerm || Object.values(filters).some((v) => v) || sortBy;

  return (
    <div className="bg-bg text-fg p-4 md:p-6 lg:p-8">
      {/* Header */}
      <div className="flex flex-col md:flex-row justify-between items-start md:items-center mb-6 space-y-4 md:space-y-0">
        <div className="flex items-center space-x-4">
          <button
            onClick={onBack}
            className="text-gray-600 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200"
            style={{ color: 'var(--color-brand)' }}
          >
            ← Back
          </button>
          <div>
            <h1 className="text-2xl font-bold">{title}</h1>
            {description && (
              <p className="text-gray-600 dark:text-gray-400 mt-1">
                {description}
              </p>
            )}
          </div>
        </div>

        {/* Action buttons */}
        <div className="flex items-center space-x-2">
          {hasActiveFilters && (
            <button
              onClick={handleClearAll}
              className="px-3 py-2 text-sm bg-gray-100 dark:bg-gray-800 hover:bg-gray-200 dark:hover:bg-gray-700 rounded-md"
            >
              Clear All
            </button>
          )}

          {enableExport && (
            <div className="flex items-center space-x-2">
              <button
                onClick={() => handleExport('csv')}
                disabled={isExporting}
                className="px-3 py-2 text-sm bg-blue-100 dark:bg-blue-900 hover:bg-blue-200 dark:hover:bg-blue-800 text-blue-800 dark:text-blue-200 rounded-md disabled:opacity-50"
              >
                {isExporting ? `Exporting... ${exportProgress}%` : 'Export CSV'}
              </button>
              <button
                onClick={() => handleExport('json')}
                disabled={isExporting}
                className="px-3 py-2 text-sm bg-green-100 dark:bg-green-900 hover:bg-green-200 dark:hover:bg-green-800 text-green-800 dark:text-green-200 rounded-md disabled:opacity-50"
              >
                Export JSON
              </button>
            </div>
          )}

          {extraAction && (
            <button
              onClick={extraAction.onClick}
              className="px-4 py-2 text-sm rounded-md"
              style={{ backgroundColor: 'var(--color-brand)', color: 'white' }}
            >
              {extraAction.label}
            </button>
          )}
        </div>
      </div>

      {/* Chart */}
      {chart && <div className="mb-6">{chart}</div>}

      {/* Main Table */}
      <div className="mb-6">
        {enableVirtualization ? (
          <VirtualizedTable
            columns={enhancedColumns}
            data={rows}
            height={height}
            rowHeight={rowHeight}
            onRowClick={onRowClick}
            sortBy={sortBy}
            sortDirection={sortDirection}
            onSort={handleSort}
            filters={filters}
            onFilter={handleFilter}
            searchTerm={searchTerm}
            onSearch={enableSearch ? handleSearch : undefined}
          />
        ) : (
          <div className="bg-card text-card-fg border border-border rounded-lg overflow-hidden">
            {/* Search Bar */}
            {enableSearch && (
              <div className="p-4 border-b border-border">
                <Input
                  placeholder={searchPlaceholder}
                  value={searchTerm}
                  onChange={(e) => handleSearch(e.target.value)}
                  className="w-full"
                />
              </div>
            )}

            {/* Table */}
            <div className="overflow-x-auto">
              <table className="min-w-full">
                <thead className="bg-muted">
                  <tr>
                    {enhancedColumns.map((column) => (
                      <th key={column.key} className="px-4 py-3 text-left">
                        <div className="flex flex-col space-y-1">
                          <span className="text-sm font-medium">
                            {column.label}
                          </span>
                          {column.filterable && (
                            <Input
                              placeholder={`Filter ${column.label}`}
                              value={filters[column.key] || ''}
                              onChange={(e) =>
                                setFilters((prev) => ({
                                  ...prev,
                                  [column.key]: e.target.value,
                                }))
                              }
                              className="px-2 py-1 text-xs"
                            />
                          )}
                        </div>
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {rows.map((row, index) => (
                    <tr
                      key={index}
                      className="border-t border-border hover:bg-muted cursor-pointer"
                      onClick={() => onRowClick?.(row)}
                    >
                      {enhancedColumns.map((column) => (
                        <td key={column.key} className="px-4 py-3 text-sm">
                          {row[column.key]}
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}
      </div>

      {/* Extra Table */}
      {extraTable && (
        <div>
          <h2 className="text-xl font-semibold mb-4">{extraTable.title}</h2>
          <div className="bg-card text-card-fg border border-border rounded-lg overflow-hidden">
            <div className="overflow-x-auto">
              <table className="min-w-full">
                <thead className="bg-gray-50 dark:bg-gray-800">
                  <tr>
                    {extraTable.columns.map((column) => (
                      <th
                        key={column.key}
                        className="px-4 py-3 text-left text-sm font-medium text-gray-900 dark:text-gray-100"
                      >
                        {column.label}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {extraTable.rows.map((row, index) => (
                    <tr
                      key={index}
                      className="border-t border-border hover:bg-muted cursor-pointer"
                      onClick={() => extraTable.onRowClick?.(row)}
                    >
                      {extraTable.columns.map((column) => (
                        <td key={column.key} className="px-4 py-3 text-sm">
                          {row[column.key]}
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      )}

      {/* Keyboard shortcuts help */}
      <div className="mt-6 text-xs text-gray-500 dark:text-gray-400">
        <p>
          Keyboard shortcuts: ESC (back), Ctrl+F (search), Ctrl+E (export CSV),
          Ctrl+R (clear filters)
        </p>
      </div>
    </div>
  );
};
