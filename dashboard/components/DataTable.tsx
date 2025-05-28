import React from 'react';

interface Column {
  key: string;
  label: string;
}

interface ExtraTable {
  title: string;
  columns: Column[];
  rows: Array<Record<string, string | number>>;
  onRowClick?: (row: Record<string, string | number>) => void;
}

interface DataTableProps {
  title: string;
  columns: Column[];
  rows: Array<Record<string, string | number>>;
  onBack: () => void;
  onRowClick?: (row: Record<string, string | number>) => void;
  extraAction?: { label: string; onClick: () => void };
  extraTable?: ExtraTable;
}

export const DataTable: React.FC<DataTableProps> = ({
  title,
  columns,
  rows,
  onBack,
  onRowClick,
  extraAction,
  extraTable,
}) => {
  return (
    <div className="p-4">
      <button
        onClick={onBack}
        className="mb-4 text-[#e81899] flex items-center space-x-1"
      >
        <span>&larr;</span>
        <span>Back</span>
      </button>
      {extraAction && (
        <button
          onClick={extraAction.onClick}
          className="ml-4 mb-4 text-[#e81899]"
        >
          {extraAction.label}
        </button>
      )}
      <h2 className="text-xl font-semibold mb-2">{title}</h2>
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
            {rows.map((row, idx) => (
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
        </div>
      ) : null}
    </div>
  );
};
