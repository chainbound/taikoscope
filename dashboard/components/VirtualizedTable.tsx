import React, { useMemo, useState, useCallback } from 'react';
import { FixedSizeList as List } from 'react-window';
import { Input } from './ui/Input';

interface Column {
  key: string;
  label: string;
  width?: number;
  sortable?: boolean;
  filterable?: boolean;
}

interface VirtualizedTableProps {
  columns: Column[];
  data: Array<Record<string, React.ReactNode | string | number>>;
  height?: number;
  rowHeight?: number;
  onRowClick?: (row: Record<string, React.ReactNode | string | number>) => void;
  sortBy?: string;
  sortDirection?: 'asc' | 'desc';
  onSort?: (column: string, direction: 'asc' | 'desc') => void;
  filters?: Record<string, string>;
  onFilter?: (filters: Record<string, string>) => void;
  searchTerm?: string;
  onSearch?: (term: string) => void;
}

const VirtualizedTable: React.FC<VirtualizedTableProps> = ({
  columns,
  data,
  height = 400,
  rowHeight = 40,
  onRowClick,
  sortBy,
  sortDirection,
  onSort,
  filters = {},
  onFilter,
  searchTerm = '',
  onSearch,
}) => {
  // Use filters prop as single source of truth for column filters
  const [localFilters, setLocalFilters] =
    useState<Record<string, string>>(filters);

  // Sync localFilters with filters prop when prop changes
  React.useEffect(() => {
    setLocalFilters(filters);
  }, [filters]);

  // Filter and search data
  const filteredData = useMemo(() => {
    let filtered = data;

    // Apply search
    if (searchTerm) {
      filtered = filtered.filter((row) =>
        Object.values(row).some((value) =>
          String(value).toLowerCase().includes(searchTerm.toLowerCase()),
        ),
      );
    }

    // Apply column filters
    Object.entries(localFilters).forEach(([column, filterValue]) => {
      if (filterValue) {
        filtered = filtered.filter((row) =>
          String(row[column]).toLowerCase().includes(filterValue.toLowerCase()),
        );
      }
    });

    return filtered;
  }, [data, searchTerm, localFilters]);

  // Sort data
  const sortedData = useMemo(() => {
    if (!sortBy) return filteredData;

    return [...filteredData].sort((a, b) => {
      const aValue = String(a[sortBy]);
      const bValue = String(b[sortBy]);

      const comparison = aValue.localeCompare(bValue, undefined, {
        numeric: true,
      });
      return sortDirection === 'desc' ? -comparison : comparison;
    });
  }, [filteredData, sortBy, sortDirection]);

  const handleSort = useCallback(
    (column: string) => {
      if (!onSort) return;

      const newDirection =
        sortBy === column && sortDirection === 'asc' ? 'desc' : 'asc';
      onSort(column, newDirection);
    },
    [sortBy, sortDirection, onSort],
  );

  const handleFilterChange = useCallback(
    (column: string, value: string) => {
      const newFilters = { ...localFilters, [column]: value };
      setLocalFilters(newFilters);
      onFilter?.(newFilters);
    },
    [localFilters, onFilter],
  );

  const Row = ({
    index,
    style,
  }: {
    index: number;
    style: React.CSSProperties;
  }) => {
    const row = sortedData[index];
    const isEven = index % 2 === 0;

    return (
      <div
        style={style}
        className={`flex items-center border-b border-gray-100 dark:border-border cursor-pointer hover:bg-muted ${
          isEven ? 'bg-card' : 'bg-muted'
        }`}
        onClick={() => onRowClick?.(row)}
      >
        {columns.map((column) => (
          <div
            key={column.key}
            className="px-4 py-2 text-sm truncate"
            style={{
              width: column.width || `${100 / columns.length}%`,
              minWidth: column.width || 120,
            }}
            title={String(row[column.key])}
          >
            {row[column.key]}
          </div>
        ))}
      </div>
    );
  };

  return (
    <div className="bg-card text-card-fg border rounded-lg border-slate-300 dark:border-slate-700">
      {/* Search Bar */}
      {onSearch && (
        <div className="p-4 border-b border-gray-100 dark:border-border">
          <Input
            placeholder="Search table data..."
            value={searchTerm}
            onChange={(e) => onSearch(e.target.value)}
            className="w-full"
          />
        </div>
      )}

      {/* Header */}
      <div className="flex bg-muted border-b border-gray-100 dark:border-border">
        {columns.map((column) => (
          <div
            key={column.key}
            className="px-4 py-3"
            style={{
              width: column.width || `${100 / columns.length}%`,
              minWidth: column.width || 120,
            }}
          >
            <div className="flex flex-col space-y-2">
              {/* Column Header */}
              <div className="flex items-center space-x-1">
                <span
                  className={`text-sm font-medium ${
                    column.sortable
                      ? 'cursor-pointer hover:text-blue-600 dark:hover:text-blue-400'
                      : ''
                  }`}
                  onClick={
                    column.sortable ? () => handleSort(column.key) : undefined
                  }
                >
                  {column.label}
                </span>
                {column.sortable && sortBy === column.key && (
                  <span className="text-blue-600 dark:text-blue-400">
                    {sortDirection === 'asc' ? '↑' : '↓'}
                  </span>
                )}
              </div>

              {/* Column Filter */}
              {column.filterable && (
                <Input
                  placeholder={`Filter ${column.label}`}
                  value={localFilters[column.key] || ''}
                  onChange={(e) =>
                    handleFilterChange(column.key, e.target.value)
                  }
                  className="px-2 py-1 text-xs"
                  onClick={(e) => e.stopPropagation()}
                />
              )}
            </div>
          </div>
        ))}
      </div>

      {/* Virtual List */}
      <List
        height={height}
        width="100%"
        itemCount={sortedData.length}
        itemSize={rowHeight}
        itemData={sortedData}
      >
        {Row}
      </List>

      {/* Footer with stats */}
      <div className="px-4 py-2 bg-muted border-t border-gray-100 dark:border-border text-sm text-gray-600 dark:text-gray-400">
        Showing {sortedData.length} of {data.length} rows
        {searchTerm && ` (filtered by "${searchTerm}")`}
      </div>
    </div>
  );
};

export default VirtualizedTable;
