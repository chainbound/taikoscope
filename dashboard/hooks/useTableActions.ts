import React, { useState, useCallback } from 'react';
import { useLocation } from 'react-router-dom';
import { TimeRange } from '../types';
import { TABLE_CONFIGS } from '../config/tableConfig';
import { getSequencerAddress } from '../sequencerConfig';
import { useRouterNavigation } from './useRouterNavigation';
import { blockLink } from '../utils';
import { fetchBlockTransactions } from '../services/apiService';

export interface TableViewState {
  title: string;
  description?: React.ReactNode;
  columns: { key: string; label: string }[];
  rows: Record<string, React.ReactNode | string | number>[];
  onRowClick?: (row: Record<string, React.ReactNode | string | number>) => void;
  extraAction?: { label: string; onClick: () => void };
  extraTable?: {
    title: string;
    columns: { key: string; label: string }[];
    rows: Record<string, React.ReactNode | string | number>[];
    onRowClick?: (
      row: Record<string, React.ReactNode | string | number>,
    ) => void;
    pagination?: {
      page: number;
      onNext: () => void;
      onPrev: () => void;
      disableNext?: boolean;
      disablePrev?: boolean;
    };
  };
  timeRange?: TimeRange;
  onTimeRangeChange?: (range: TimeRange) => void;
  onRefresh?: () => void;
  chart?: React.ReactNode;
  allRows?: Record<string, React.ReactNode | string | number>[];
  useClientSidePagination?: boolean;
  totalRecords?: number;
}

export const useTableActions = (
  timeRange: TimeRange,
  setTimeRange: (range: TimeRange) => void,
  selectedSequencer: string | null,
) => {
  const [tableView, setTableView] = useState<TableViewState | null>(null);
  const [tableLoading, setTableLoading] = useState<boolean>(false);
  const [seqDistTxPage, setSeqDistTxPage] = useState<number>(0);
  const { navigateToTable } = useRouterNavigation();
  const location = useLocation();

  const setTableUrl = useCallback(
    (
      name: string,
      params: Record<string, string | number | undefined> = {},
    ) => {
      try {
        const cleanParams: Record<string, string | number> = {};
        Object.entries(params).forEach(([k, v]) => {
          if (v !== undefined) cleanParams[k] = v;
        });

        navigateToTable(name, cleanParams, timeRange);
      } catch (err) {
        console.error('Failed to set table URL:', err);
      }
    },
    [navigateToTable, timeRange],
  );

  const openTable = useCallback(
    (
      title: string,
      description: React.ReactNode | undefined,
      columns: { key: string; label: string }[],
      rows: Record<string, React.ReactNode | string | number>[],
      onRowClick?: (
        row: Record<string, React.ReactNode | string | number>,
      ) => void,
      extraAction?: { label: string; onClick: () => void },
      extraTable?: TableViewState['extraTable'],
      range?: TimeRange,
      onRangeChange?: (range: TimeRange) => void,
      onRefresh?: () => void,
      chart?: React.ReactNode,
      allRows?: Record<string, React.ReactNode | string | number>[],
      useClientSidePagination?: boolean,
      totalRecords?: number,
    ) => {
      setTableView({
        title,
        description,
        columns,
        rows,
        onRowClick,
        extraAction,
        extraTable,
        timeRange: range,
        onTimeRangeChange: onRangeChange,
        onRefresh,
        chart,
        allRows,
        useClientSidePagination,
        totalRecords,
      });
      setTableLoading(false);
    },
    [],
  );

  const openGenericTable = useCallback(
    async (
      tableKey: string,
      range: TimeRange = timeRange,
      extraParams: Record<string, any> = {},
    ) => {
      const config = TABLE_CONFIGS[tableKey];
      if (!config) return;

      const onTableRoute = location.pathname.startsWith('/table/');

      if (!onTableRoute) {
        setTableUrl(config.urlKey, { range, ...extraParams });
        return;
      }

      setTableLoading(true);
      setTimeRange(range);

      try {
        const fetcherArgs: any[] = [];
        if (tableKey === 'sequencer-blocks' && extraParams.address) {
          fetcherArgs.push(extraParams.address);
        } else if (
          ['l2-block-times', 'l2-gas-used', 'l2-tps'].includes(tableKey)
        ) {
          fetcherArgs.push(
            selectedSequencer
              ? getSequencerAddress(selectedSequencer)
              : undefined,
          );
        }

        const res = await config.fetcher(range, ...fetcherArgs);
        const data = res.data || [];

        const title =
          typeof config.title === 'function'
            ? config.title(extraParams)
            : config.title;

        const mappedData = config.mapData
          ? config.mapData(data, extraParams)
          : data;
        const chart = config.chart ? config.chart(data) : undefined;

        setTableUrl(config.urlKey, { range, ...extraParams });

        // Create a refresh function that fetches fresh data
        const refreshData = async () => {
          try {
            console.log(`Refreshing ${tableKey} table data`);
            const refreshRes = await config.fetcher(range, ...fetcherArgs);
            const refreshDataResult = refreshRes.data || [];
            const refreshMappedData = config.mapData
              ? config.mapData(refreshDataResult, extraParams)
              : refreshDataResult;
            const refreshChart = config.chart
              ? config.chart(refreshDataResult)
              : undefined;

            setTableView((prev) =>
              prev
                ? {
                    ...prev,
                    rows: prev.useClientSidePagination
                      ? prev.rows
                      : refreshMappedData,
                    allRows: prev.useClientSidePagination
                      ? refreshMappedData
                      : undefined,
                    chart: refreshChart,
                    totalRecords: prev.useClientSidePagination
                      ? refreshMappedData.length
                      : undefined,
                  }
                : null,
            );
          } catch (error) {
            console.error(`Failed to refresh ${tableKey} table:`, error);
            // Optionally show user-facing error
            setTableView((prev) =>
              prev
                ? {
                    ...prev,
                    rows: [], // Clear data on error to prevent stale data
                    allRows: prev.useClientSidePagination ? [] : undefined,
                    totalRecords: 0,
                  }
                : null,
            );
          }
        };

        // Check if this table should use unlimited data (for chart consistency)
        const useUnlimitedData = config.useUnlimitedData || false;

        openTable(
          title,
          config.description,
          config.columns,
          useUnlimitedData ? mappedData.slice(0, 50) : mappedData, // Show first 50 for display
          tableKey === 'sequencer-dist'
            ? (row) =>
                openGenericTable('sequencer-blocks', range, {
                  address: row.name,
                })
            : undefined,
          undefined,
          undefined,
          range,
          (r) => openGenericTable(tableKey, r, extraParams),
          refreshData,
          chart,
          useUnlimitedData ? mappedData : undefined, // Pass full dataset for client-side pagination
          useUnlimitedData, // Enable client-side pagination flag
          useUnlimitedData ? mappedData.length : undefined, // Total record count
        );
      } catch (error) {
        console.error(`Failed to open ${tableKey} table:`, error);
        setTableLoading(false);
      }
    },
    [
      timeRange,
      setTimeRange,
      selectedSequencer,
      openTable,
      setTableUrl,
      setTableView,
      location.pathname,
    ],
  );

  const openSequencerDistributionTable = useCallback(
    async (
      range: TimeRange,
      page = seqDistTxPage,
      startingAfter?: number,
      endingBefore?: number,
    ) => {
      setTableLoading(true);
      setTimeRange(range);
      setSeqDistTxPage(page);
      setTableUrl('sequencer-dist', {
        range,
        page,
        start: startingAfter,
        end: endingBefore,
      });

      const [distRes, txRes] = await Promise.all([
        TABLE_CONFIGS['sequencer-dist'].fetcher(range),
        fetchBlockTransactions(
          range,
          50,
          startingAfter,
          endingBefore,
          selectedSequencer
            ? getSequencerAddress(selectedSequencer)
            : undefined,
        ),
      ]);

      const txData = txRes.data || [];
      const disablePrev = page === 0;
      const disableNext = txData.length < 50;
      const nextCursor =
        txData.length > 0 ? txData[txData.length - 1].block : undefined;
      const prevCursor = txData.length > 0 ? txData[0].block : undefined;

      const refreshSeqDist = async () => {
        try {
          const [refreshDistRes, refreshTxRes] = await Promise.all([
            TABLE_CONFIGS['sequencer-dist'].fetcher(range),
            fetchBlockTransactions(
              range,
              50,
              undefined, // Reset pagination on refresh
              undefined,
              selectedSequencer
                ? getSequencerAddress(selectedSequencer)
                : undefined,
            ),
          ]);

          setTableView((prev) =>
            prev
              ? {
                  ...prev,
                  rows: (refreshDistRes.data || []) as unknown as Record<
                    string,
                    string | number
                  >[],
                  extraTable: prev.extraTable
                    ? {
                        ...prev.extraTable,
                        rows: (refreshTxRes.data || []).map((t) => ({
                          block: blockLink(t.block),
                          txs: t.txs,
                          sequencer: t.sequencer,
                        })) as unknown as Record<
                          string,
                          React.ReactNode | string | number
                        >[],
                      }
                    : undefined,
                }
              : null,
          );
        } catch (error) {
          console.error(
            'Failed to refresh sequencer distribution table:',
            error,
          );
          // Clear data on error to prevent showing stale information
          setTableView((prev) =>
            prev
              ? {
                  ...prev,
                  rows: [],
                  extraTable: prev.extraTable
                    ? {
                        ...prev.extraTable,
                        rows: [],
                      }
                    : undefined,
                }
              : null,
          );
        }
      };

      openTable(
        'Sequencer Distribution',
        'Breakdown of blocks proposed by each sequencer.',
        [
          { key: 'name', label: 'Sequencer' },
          { key: 'value', label: 'Blocks' },
        ],
        (distRes.data || []) as unknown as Record<
          string,
          React.ReactNode | string | number
        >[],
        (row) => {
          const cleanParams: Record<string, string | number> = {
            address: String(row.name),
          };
          navigateToTable('sequencer-blocks', cleanParams, range);
        },
        undefined,
        {
          title: 'Transactions',
          columns: [
            { key: 'block', label: 'L2 Block Number' },
            { key: 'txs', label: 'Tx Count' },
            { key: 'sequencer', label: 'Sequencer' },
          ],
          rows: (txRes.data || []).map((t) => ({
            block: blockLink(t.block),
            txs: t.txs,
            sequencer: t.sequencer,
          })) as unknown as Record<string, React.ReactNode | string | number>[],
          pagination: {
            page,
            onPrev: () =>
              openSequencerDistributionTable(
                range,
                page - 1,
                undefined,
                prevCursor,
              ),
            onNext: () =>
              openSequencerDistributionTable(
                range,
                page + 1,
                nextCursor,
                undefined,
              ),
            disablePrev,
            disableNext,
          },
        },
        range,
        (r) => openSequencerDistributionTable(r, 0),
        refreshSeqDist,
        undefined,
      );
    },
    [
      seqDistTxPage,
      setTimeRange,
      setSeqDistTxPage,
      setTableUrl,
      selectedSequencer,
      openTable,
      openGenericTable,
    ],
  );

  return {
    tableView,
    tableLoading,
    setTableView,
    setTableLoading,
    openGenericTable,
    openSequencerDistributionTable,
  };
};
