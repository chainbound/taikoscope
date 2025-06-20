import React from 'react';
import { TAIKO_PINK } from '../theme';

interface MetricCardProps {
  title: React.ReactNode;
  value: string;
  link?: string;
  unit?: string; // Unit is passed but not displayed in the title directly as (unit)
  description?: React.ReactNode;
  className?: string;
  onMore?: () => void;
}

export const MetricCard: React.FC<MetricCardProps> = ({
  title,
  value,
  link,
  description,
  className,
  onMore,
}) => {
  // Check if value looks like an Ethereum address (0x followed by 40 hex characters)
  const isAddress = /^0x[a-fA-F0-9]{40}$/.test(value);
  const isShortValue = !isAddress && value.length <= 16;

  return (
    <div
      className={`bg-white dark:bg-gray-800 p-4 rounded-lg border border-gray-200 dark:border-gray-700 transition-shadow duration-200 ${isAddress ? 'min-w-0 w-full col-span-2 sm:col-span-2 md:col-span-2 lg:col-span-2 xl:col-span-2 2xl:col-span-2' : ''} ${className ?? ''}`.trim()}
    >
      <div className="relative">
        <h3 className="text-xs sm:text-sm font-medium text-gray-500 dark:text-gray-400 truncate pr-8">
          {title}
        </h3>
        {onMore && (
          <button
            onClick={onMore}
            className="absolute top-0 right-0 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300 text-xl leading-none w-6 h-6 flex items-center justify-center rounded-md"
            aria-label="View table"
          >
            ⋮
          </button>
        )}
      </div>
      <p
        className={`mt-1 font-semibold text-gray-900 dark:text-gray-100 ${isAddress ? 'text-base break-all' : `text-2xl${isShortValue ? '' : ' whitespace-nowrap overflow-hidden text-ellipsis'}`}`}
      >
        {link ? (
          <a
            href={link}
            target="_blank"
            rel="noopener noreferrer"
            className="hover:underline"
            style={{ color: TAIKO_PINK }}
          >
            {value}
          </a>
        ) : (
          value
        )}
      </p>
      {description && (
        <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
          {description}
        </p>
      )}
    </div>
  );
};
