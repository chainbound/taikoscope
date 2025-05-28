import React from 'react';

interface Column {
  key: string;
  label: string;
}

interface DataTableProps {
  title: string;
  columns: Column[];
  rows: Array<Record<string, string | number>>;
  onBack: () => void;
}

export const DataTable: React.FC<DataTableProps> = ({
  title,
  columns,
  rows,
  onBack,
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
              <tr key={idx} className="border-t">
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
    </div>
  );
};
