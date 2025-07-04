import React from 'react';
import { useSearchParams } from 'react-router-dom';
import { useRouterNavigation } from '../hooks/useRouterNavigation';
import { TAIKO_PINK } from '../theme';

export const ViewTabs: React.FC = () => {
  const { updateSearchParams, navigateToDashboard } = useRouterNavigation();
  const [searchParams] = useSearchParams();
  const view = searchParams.get('view') || 'economics';

  const tabs: { label: string; value: string }[] = [
    { label: 'Performance', value: 'performance' },
    { label: 'Economics', value: 'economics' },
    { label: 'Health', value: 'health' },
  ];

  return (
    <div className="flex space-x-2">
      {tabs.map((t) => (
        <button
          key={t.value}
          onClick={() => {
            if (t.value === 'economics' && view === 'economics') {
              navigateToDashboard(true);
              return;
            }
            updateSearchParams({ view: t.value, table: null });
          }}
          className={`px-2 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 hover:bg-gray-100 dark:hover:bg-gray-700 ${view === t.value ? 'font-semibold' : ''}`}
          style={{ color: TAIKO_PINK }}
        >
          {t.label}
        </button>
      ))}
    </div>
  );
};
