import React from 'react';
import type { BlockTransaction } from '../services/apiService';

interface SequencerBlockTableProps {
  data: BlockTransaction[];
}

export const SequencerBlockTable: React.FC<SequencerBlockTableProps> = ({ data }) => {
  if (!data || data.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No data available
      </div>
    );
  }

  return (
    <div className="overflow-x-auto h-full">
      <table className="min-w-full border divide-y divide-gray-200 text-sm">
        <thead>
          <tr>
            <th className="px-2 py-1 text-left">Block Number</th>
            <th className="px-2 py-1 text-left">Tx Count</th>
          </tr>
        </thead>
        <tbody>
          {data.map((row, idx) => (
            <tr key={idx} className="border-t">
              <td className="px-2 py-1">{row.block}</td>
              <td className="px-2 py-1">{row.txs}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};
