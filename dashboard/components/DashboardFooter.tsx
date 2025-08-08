import React from 'react';
import { TAIKOSCAN_BASE, ETHERSCAN_BASE } from '../utils';

interface DashboardFooterProps {
  l2HeadBlock: string;
  l1HeadBlock: string;
}

export const DashboardFooter: React.FC<DashboardFooterProps> = ({
  l2HeadBlock,
  l1HeadBlock,
}) => (
  <footer className="mt-8 px-4 py-6 md:px-6 lg:px-8 border-t border-gray-100 dark:border-border">
    <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-center">
      <div>
        <span className="text-sm text-gray-500 dark:text-gray-400">
          L2 Head Block
        </span>
        <p className="text-2xl font-semibold text-brand">
          {Number.isFinite(Number(l2HeadBlock.replace(/,/g, ''))) ? (
            <a
              href={`${TAIKOSCAN_BASE}/block/${Number(
                l2HeadBlock.replace(/,/g, ''),
              )}`}
              target="_blank"
              rel="noopener noreferrer"
              className="hover:underline text-brand"
            >
              {l2HeadBlock}
            </a>
          ) : (
            l2HeadBlock
          )}
        </p>
      </div>
      <div>
        <span className="text-sm text-gray-500 dark:text-gray-400">
          L1 Head Block
        </span>
        <p className="text-2xl font-semibold text-brand">
          {Number.isFinite(Number(l1HeadBlock.replace(/,/g, ''))) ? (
            <a
              href={`${ETHERSCAN_BASE}/block/${Number(
                l1HeadBlock.replace(/,/g, ''),
              )}`}
              target="_blank"
              rel="noopener noreferrer"
              className="hover:underline text-brand"
            >
              {l1HeadBlock}
            </a>
          ) : (
            l1HeadBlock
          )}
        </p>
      </div>
    </div>
    <div className="mt-4 text-sm text-gray-500 dark:text-gray-400 text-center">
      Made by{' '}
      <a
        href="https://chainbound.io/"
        target="_blank"
        rel="noopener noreferrer"
        className="hover:underline text-brand"
      >
        Chainbound
      </a>
      <span className="mx-2">|</span>
      <a
        href="https://x.com/chainbound_"
        target="_blank"
        rel="noopener noreferrer"
        className="hover:underline text-brand"
      >
        X
      </a>
      <span className="mx-2">|</span>
      <a
        href="https://github.com/chainbound/taikoscope/"
        target="_blank"
        rel="noopener noreferrer"
        className="hover:underline text-brand"
      >
        GitHub
      </a>
    </div>
  </footer>
);
