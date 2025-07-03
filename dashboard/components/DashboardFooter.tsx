import React from 'react';
import { TAIKO_PINK } from '../theme';
import { TAIKOSCAN_BASE, ETHERSCAN_BASE } from '../utils';

interface DashboardFooterProps {
  l2HeadBlock: string;
  l1HeadBlock: string;
}

export const DashboardFooter: React.FC<DashboardFooterProps> = ({
  l2HeadBlock,
  l1HeadBlock,
}) => (
  <footer className="mt-8 px-4 py-6 md:px-6 lg:px-8 border-t border-gray-200 dark:border-gray-700">
    <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-center md:text-left">
      <div>
        <span className="text-sm text-gray-500 dark:text-gray-400">
          L2 Head Block
        </span>
        <p className="text-2xl font-semibold" style={{ color: TAIKO_PINK }}>
          {Number.isFinite(Number(l2HeadBlock.replace(/,/g, ''))) ? (
            <a
              href={`${TAIKOSCAN_BASE}/block/${Number(
                l2HeadBlock.replace(/,/g, ''),
              )}`}
              target="_blank"
              rel="noopener noreferrer"
              className="hover:underline"
              style={{ color: TAIKO_PINK }}
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
        <p className="text-2xl font-semibold" style={{ color: TAIKO_PINK }}>
          {Number.isFinite(Number(l1HeadBlock.replace(/,/g, ''))) ? (
            <a
              href={`${ETHERSCAN_BASE}/block/${Number(
                l1HeadBlock.replace(/,/g, ''),
              )}`}
              target="_blank"
              rel="noopener noreferrer"
              className="hover:underline"
              style={{ color: TAIKO_PINK }}
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
      Made by Chainbound
      <span className="mx-2">|</span>
      <a
        href="https://x.com/chainbound_"
        target="_blank"
        rel="noopener noreferrer"
        className="hover:underline"
        style={{ color: TAIKO_PINK }}
      >
        X
      </a>
      <span className="mx-1">|</span>
      <a
        href="https://github.com/chainbound/taikoscope/"
        target="_blank"
        rel="noopener noreferrer"
        className="hover:underline"
        style={{ color: TAIKO_PINK }}
      >
        GitHub
      </a>
    </div>
  </footer>
);
