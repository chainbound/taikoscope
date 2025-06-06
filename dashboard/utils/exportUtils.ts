export type ExportFormat = 'csv' | 'json' | 'excel';

export interface ExportOptions {
  filename?: string;
  includeHeaders?: boolean;
  selectedColumns?: string[];
  dateFormat?: string;
}

/**
 * Convert table data to CSV format
 */
export const convertToCSV = (
  data: Array<Record<string, any>>,
  columns: Array<{ key: string; label: string }>,
  options: ExportOptions = {},
): string => {
  const { includeHeaders = true, selectedColumns } = options;

  // Filter columns if specific ones are selected
  const columnsToExport = selectedColumns
    ? columns.filter((col) => selectedColumns.includes(col.key))
    : columns;

  const rows: string[] = [];

  // Add headers if requested
  if (includeHeaders) {
    const headers = columnsToExport.map((col) => `"${col.label}"`).join(',');
    rows.push(headers);
  }

  // Add data rows
  data.forEach((row) => {
    const values = columnsToExport.map((col) => {
      const value = row[col.key];

      // Handle different data types
      if (value === null || value === undefined) {
        return '""';
      }

      // Convert React nodes to string
      if (typeof value === 'object' && value.toString) {
        return `"${String(value).replace(/"/g, '""')}"`;
      }

      // Escape quotes in strings
      const stringValue = String(value).replace(/"/g, '""');
      return `"${stringValue}"`;
    });

    rows.push(values.join(','));
  });

  return rows.join('\n');
};

/**
 * Convert table data to JSON format
 */
export const convertToJSON = (
  data: Array<Record<string, any>>,
  columns: Array<{ key: string; label: string }>,
  options: ExportOptions = {},
): string => {
  const { selectedColumns } = options;

  // Filter columns if specific ones are selected
  const columnsToExport = selectedColumns
    ? columns.filter((col) => selectedColumns.includes(col.key))
    : columns;

  const exportData = data.map((row) => {
    const exportRow: Record<string, any> = {};

    columnsToExport.forEach((col) => {
      let value = row[col.key];

      // Convert React nodes to string
      if (typeof value === 'object' && value !== null && value.toString) {
        value = String(value);
      }

      exportRow[col.label] = value;
    });

    return exportRow;
  });

  return JSON.stringify(exportData, null, 2);
};

/**
 * Download data as a file
 */
export const downloadFile = (
  content: string,
  filename: string,
  mimeType: string,
): void => {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);

  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  link.style.display = 'none';

  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);

  // Clean up the URL object
  URL.revokeObjectURL(url);
};

/**
 * Generate filename with timestamp
 */
export const generateFilename = (
  baseName: string,
  format: ExportFormat,
  timestamp: boolean = true,
): string => {
  const now = new Date();
  const dateStr = timestamp
    ? `_${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}_${String(now.getHours()).padStart(2, '0')}-${String(now.getMinutes()).padStart(2, '0')}`
    : '';

  return `${baseName}${dateStr}.${format}`;
};

/**
 * Main export function
 */
export const exportTableData = (
  data: Array<Record<string, any>>,
  columns: Array<{ key: string; label: string }>,
  format: ExportFormat,
  options: ExportOptions = {},
): void => {
  const { filename } = options;

  let content: string;
  let mimeType: string;
  let defaultFilename: string;

  switch (format) {
    case 'csv':
      content = convertToCSV(data, columns, options);
      mimeType = 'text/csv;charset=utf-8;';
      defaultFilename = generateFilename(filename || 'table-data', 'csv');
      break;

    case 'json':
      content = convertToJSON(data, columns, options);
      mimeType = 'application/json;charset=utf-8;';
      defaultFilename = generateFilename(filename || 'table-data', 'json');
      break;

    default:
      throw new Error(`Unsupported export format: ${format}`);
  }

  downloadFile(content, defaultFilename, mimeType);
};

/**
 * Export utility with progress tracking for large datasets
 */
export const exportLargeDataset = async (
  data: Array<Record<string, any>>,
  columns: Array<{ key: string; label: string }>,
  format: ExportFormat,
  options: ExportOptions = {},
  onProgress?: (progress: number) => void,
): Promise<void> => {
  const chunkSize = 1000;
  const totalRows = data.length;

  return new Promise((resolve, reject) => {
    try {
      if (data.length <= chunkSize) {
        // Small dataset, export normally
        exportTableData(data, columns, format, options);
        onProgress?.(100);
        resolve();
        return;
      }

      // Large dataset, process in chunks
      let processedRows = 0;
      const chunks: string[] = [];

      // Add headers for CSV
      if (format === 'csv' && options.includeHeaders !== false) {
        const columnsToExport = options.selectedColumns
          ? columns.filter((col) => options.selectedColumns!.includes(col.key))
          : columns;
        const headers = columnsToExport
          .map((col) => `"${col.label}"`)
          .join(',');
        chunks.push(headers);
      }

      const processChunk = () => {
        const start = processedRows;
        const end = Math.min(start + chunkSize, totalRows);
        const chunk = data.slice(start, end);

        let chunkContent: string;

        if (format === 'csv') {
          chunkContent = convertToCSV(chunk, columns, {
            ...options,
            includeHeaders: false,
          });
        } else {
          // For JSON, we'll need to handle this differently for large datasets
          chunkContent = convertToJSON(chunk, columns, options);
        }

        chunks.push(chunkContent);
        processedRows = end;

        const progress = Math.round((processedRows / totalRows) * 100);
        onProgress?.(progress);

        if (processedRows < totalRows) {
          // Process next chunk asynchronously
          setTimeout(processChunk, 10);
        } else {
          // All chunks processed, combine and download
          let finalContent: string;

          if (format === 'json') {
            // Combine JSON chunks properly
            const allData = data.map((row) => {
              const exportRow: Record<string, any> = {};
              columns.forEach((col) => {
                let value = row[col.key];
                if (
                  typeof value === 'object' &&
                  value !== null &&
                  value.toString
                ) {
                  value = String(value);
                }
                exportRow[col.label] = value;
              });
              return exportRow;
            });
            finalContent = JSON.stringify(allData, null, 2);
          } else {
            finalContent = chunks.join('\n');
          }

          const filename = generateFilename(
            options.filename || 'table-data',
            format,
          );
          const mimeType =
            format === 'csv'
              ? 'text/csv;charset=utf-8;'
              : 'application/json;charset=utf-8;';

          downloadFile(finalContent, filename, mimeType);
          resolve();
        }
      };

      // Start processing
      processChunk();
    } catch (error) {
      reject(error);
    }
  });
};

/**
 * Validate export data
 */
export const validateExportData = (
  data: Array<Record<string, any>>,
  columns: Array<{ key: string; label: string }>,
): { isValid: boolean; errors: string[] } => {
  const errors: string[] = [];

  if (!data || data.length === 0) {
    errors.push('No data to export');
  }

  if (!columns || columns.length === 0) {
    errors.push('No columns defined for export');
  }

  // Check if columns exist in data
  if (data.length > 0 && columns.length > 0) {
    const dataKeys = Object.keys(data[0]);
    const missingColumns = columns
      .map((col) => col.key)
      .filter((key) => !dataKeys.includes(key));

    if (missingColumns.length > 0) {
      errors.push(`Missing columns in data: ${missingColumns.join(', ')}`);
    }
  }

  return {
    isValid: errors.length === 0,
    errors,
  };
};
