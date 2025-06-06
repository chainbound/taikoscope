import { useCallback } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { safeNavigate } from '../utils/navigationUtils';
import { TimeRange } from '../types';

export const useRouterNavigation = () => {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();

  const navigateToTable = useCallback(
    (
      tableType: string,
      params?: Record<string, string | number>,
      range?: TimeRange,
    ) => {
      const queryParams = new URLSearchParams();

      if (range) {
        queryParams.set('range', range);
      }

      if (params) {
        Object.entries(params).forEach(([key, value]) => {
          queryParams.set(key, String(value));
        });
      }

      const queryString = queryParams.toString();
      const path = `/table/${tableType}${queryString ? `?${queryString}` : ''}`;
      safeNavigate(navigate, path);
    },
    [navigate],
  );

  const navigateToSequencer = useCallback(
    (address: string) => {
      safeNavigate(navigate, `/sequencer/${address}`);
    },
    [navigate],
  );

  const navigateToDashboard = useCallback(
    (preserveParams = false) => {
      if (preserveParams) {
        const params = new URLSearchParams();
        const sequencer = searchParams.get('sequencer');
        const range = searchParams.get('range');

        if (sequencer) params.set('sequencer', sequencer);
        if (range) params.set('range', range);

        const queryString = params.toString();
        safeNavigate(navigate, `/${queryString ? `?${queryString}` : ''}`);
      } else {
        safeNavigate(navigate, '/');
      }
    },
    [navigate, searchParams],
  );

  const updateSearchParams = useCallback(
    (updates: Record<string, string | null>) => {
      const newParams = new URLSearchParams(searchParams);

      Object.entries(updates).forEach(([key, value]) => {
        if (value === null) {
          newParams.delete(key);
        } else {
          newParams.set(key, value);
        }
      });

      const queryString = newParams.toString();
      safeNavigate(navigate, `?${queryString}`, true);
    },
    [navigate, searchParams],
  );

  return {
    navigateToTable,
    navigateToSequencer,
    navigateToDashboard,
    updateSearchParams,
  };
};
