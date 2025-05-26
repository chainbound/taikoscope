import React from "react";

interface SummaryBarProps {
  l2HeadBlock: string;
  l1HeadBlock: string;
}

export const SummaryBar: React.FC<SummaryBarProps> = ({
  l2HeadBlock,
  l1HeadBlock,
}) => {
  return (
    <div className="mt-4 mb-6 bg-gray-50 border border-gray-200 rounded-md p-3 grid grid-cols-2 gap-4 text-center md:text-left">
      <div>
        <span className="text-sm text-gray-500">L2 Head Block</span>
        <p className="text-xl font-semibold text-[#e81899]">{l2HeadBlock}</p>
      </div>
      <div>
        <span className="text-sm text-gray-500">L1 Head Block</span>
        <p className="text-xl font-semibold text-[#e81899]">{l1HeadBlock}</p>
      </div>
    </div>
  );
};
