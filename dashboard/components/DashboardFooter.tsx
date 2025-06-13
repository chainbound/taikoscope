import React from 'react';
import { TAIKO_PINK } from '../theme';

interface DashboardFooterProps {
  l2HeadBlock: string;
  l1HeadBlock: string;
}

export const DashboardFooter: React.FC<DashboardFooterProps> = ({
  l2HeadBlock,
  l1HeadBlock,
}) => (
  <footer className="mt-8 px-2 sm:px-4 md:px-6 lg:px-8 py-6 border-t border-gray-200 dark:border-gray-700">
    <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-center md:text-left">
      <div>
        <span className="text-sm text-gray-500 dark:text-gray-400">
          L2 Head Block
        </span>
        <p className="text-2xl font-semibold" style={{ color: TAIKO_PINK }}>
          {l2HeadBlock}
        </p>
      </div>
      <div>
        <span className="text-sm text-gray-500 dark:text-gray-400">
          L1 Head Block
        </span>
        <p className="text-2xl font-semibold" style={{ color: TAIKO_PINK }}>
          {l1HeadBlock}
        </p>
      </div>
    </div>
  </footer>
);
