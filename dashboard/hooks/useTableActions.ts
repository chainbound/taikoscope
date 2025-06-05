import React, { useState, useCallback } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { TimeRange } from '../types';
import { TABLE_CONFIGS } from '../config/tableConfig';
import { getSequencerAddress } from '../sequencerConfig';
import {
  fetchBlockTransactions,
  type BlockTransaction,
} from '../services/apiService';

export interface TableViewState {
  title: string;
  description?: React.ReactNode;
  columns: { key: string; label: string }[];
  rows: Record<string, string | number>[];
  onRowClick?: (row: Record<string, string | number>) => void;
  extraAction?: { label: string; onClick: () => void };
  extraTable?: {
    title: string;
    columns: { key: string; label: string }[];
    rows: Record<string, string | number>[];
    onRowClick?: (row: Record<string, string | number>) => void;
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
}

export const useTableActions = (
  timeRange: TimeRange,
  setTimeRange: (range: TimeRange) => void,
  selectedSequencer: string | null,
  blockTxData: BlockTransaction[],
  l2BlockTimeData: any[],
) => {
  const [tableView, setTableView] = useState<TableViewState | null>(null);
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const [tableLoading, setTableLoading] = useState<boolean>(
    searchParams.get('view') === 'table',
  );
  const [seqDistTxPage, setSeqDistTxPage] = useState<number>(0);
  const requestTokenRef = React.useRef(0);

  const cancelPendingRequests = useCallback(() => {
    requestTokenRef.current += 1;
  }, []);

  const setTableUrl = useCallback(
    (
      name: string,
      params: Record<string, string | number | undefined> = {},
    ) => {
      try {
        const newParams = new URLSearchParams(searchParams);
        newParams.set('view', 'table');
        newParams.set('table', name);

        ['address', 'page', 'start', 'end'].forEach((key) => {
          newParams.delete(key);
        });

        Object.entries(params).forEach(([k, v]) => {
          if (v !== undefined) newParams.set(k, String(v));
        });

        navigate({ search: newParams.toString() }, { replace: false });
      } catch (err) {
        console.error('Failed to set table URL:', err);
      }
    },
    [navigate, searchParams],
  );

  const openTable = useCallback(
    (
      title: string,
      description: React.ReactNode | undefined,
      columns: { key: string; label: string }[],
      rows: Record<string, string | number>[],
      onRowClick?: (row: Record<string, string | number>) => void,
      extraAction?: { label: string; onClick: () => void },
      extraTable?: TableViewState['extraTable'],
      range?: TimeRange,
      onRangeChange?: (range: TimeRange) => void,
      onRefresh?: () => void,
      chart?: React.ReactNode,
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
      const token = ++requestTokenRef.current;
      const config = TABLE_CONFIGS[tableKey];
      if (!config) return;

      setTableLoading(true);
      setTimeRange(range);

      try {
        const fetcherArgs: any[] = [];
        if (tableKey === 'sequencer-blocks' && extraParams.address) {
          fetcherArgs.push(extraParams.address);
        } else if (['l2-block-times', 'l2-gas-used'].includes(tableKey)) {
          fetcherArgs.push(
            selectedSequencer
              ? getSequencerAddress(selectedSequencer)
              : undefined,
          );
        }

        const res = await config.fetcher(range, ...fetcherArgs);
        if (token !== requestTokenRef.current) return;
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
            if (token !== requestTokenRef.current) return;
            const refreshDataResult = refreshRes.data || [];
            const refreshMappedData = config.mapData
              ? config.mapData(refreshDataResult, extraParams)
              : refreshDataResult;
            const refreshChart = config.chart ? config.chart(refreshDataResult) : undefined;

            setTableView(prev => prev ? {
              ...prev,
              rows: refreshMappedData,
              chart: refreshChart,
            } : null);
          } catch (error) {
            console.error(`Failed to refresh ${tableKey} table:`, error);
            // Optionally show user-facing error
            setTableView(prev => prev ? {
              ...prev,
              rows: [], // Clear data on error to prevent stale data
            } : null);
          }
        };

        if (token !== requestTokenRef.current) return;

        openTable(
          title,
          tableKey === 'reorgs'
            ? 'An L2 reorg occurs when the chain replaces previously published blocks. Depth shows how many blocks were replaced.'
            : undefined,
          config.columns,
          mappedData,
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
        );
      } catch (error) {
        console.error(`Failed to open ${tableKey} table:`, error);
        setTableLoading(false);
      }
    },
    [timeRange, setTimeRange, selectedSequencer, openTable, setTableUrl, setTableView],
  );

  const openTpsTable = useCallback(() => {
    setTableLoading(true);
    setTableUrl('tps');

    const intervalMap = new Map<number, number>();
    l2BlockTimeData.forEach((d) => {
      intervalMap.set(d.value, d.timestamp);
    });

    const data = blockTxData
      .map((b) => {
        const ms = intervalMap.get(b.block);
        if (!ms) return null;
        return { block: b.block, tps: b.txs / (ms / 1000) };
      })
      .filter((d): d is { block: number; tps: number } => d !== null);

    const TpsChart = React.lazy(() =>
      import('../components/TpsChart').then((m) => ({ default: m.TpsChart })),
    );

    openTable(
      'Transactions Per Second',
      undefined,
      [
        { key: 'block', label: 'Block Number' },
        { key: 'tps', label: 'TPS' },
      ],
      data.map((d) => ({ block: d.block, tps: d.tps.toFixed(2) })) as Record<
        string,
        string | number
      >[],
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      undefined, // No refresh function for TPS table since it depends on other data
      React.createElement(TpsChart, { data, lineColor: '#4E79A7' }),
    );
  }, [blockTxData, l2BlockTimeData, openTable, setTableUrl]);

  const openSequencerDistributionTable = useCallback(
    async (
      range: TimeRange,
      page = seqDistTxPage,
      startingAfter?: number,
      endingBefore?: number,
    ) => {
      const token = ++requestTokenRef.current;
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
      if (token !== requestTokenRef.current) return;

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
          if (token !== requestTokenRef.current) return;

          setTableView(prev => token === requestTokenRef.current && prev ? {
            ...prev,
            rows: (refreshDistRes.data || []) as unknown as Record<string, string | number>[],
            extraTable: prev.extraTable ? {
              ...prev.extraTable,
              rows: (refreshTxRes.data || []) as unknown as Record<string, string | number>[],
            } : undefined,
          } : null);
        } catch (error) {
          console.error('Failed to refresh sequencer distribution table:', error);
          // Clear data on error to prevent showing stale information
          setTableView(prev => prev ? {
            ...prev,
            rows: [],
            extraTable: prev.extraTable ? {
              ...prev.extraTable,
              rows: [],
            } : undefined,
          } : null);
        }
      };

      if (token !== requestTokenRef.current) return;

      openTable(
        'Sequencer Distribution',
        undefined,
        [
          { key: 'name', label: 'Sequencer' },
          { key: 'value', label: 'Blocks' },
        ],
        (distRes.data || []) as unknown as Record<string, string | number>[],
        (row) =>
          openGenericTable('sequencer-blocks', range, { address: row.name }),
        undefined,
        {
          title: 'Transactions',
          columns: [
            { key: 'block', label: 'Block Number' },
            { key: 'txs', label: 'Tx Count' },
            { key: 'sequencer', label: 'Sequencer' },
          ],
          rows: (txRes.data || []) as unknown as Record<
            string,
            string | number
          >[],
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
    cancelPendingRequests,
    openGenericTable,
    openTpsTable,
    openSequencerDistributionTable,
  };
};
