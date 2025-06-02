import React, { useState, useCallback } from 'react';
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
  const [tableLoading, setTableLoading] = useState<boolean>(
    new URLSearchParams(window.location.search).get('view') === 'table',
  );
  const [seqDistTxPage, setSeqDistTxPage] = useState<number>(0);

  const setTableUrl = useCallback(
    (
      name: string,
      params: Record<string, string | number | undefined> = {},
    ) => {
      const url = new URL(window.location.href);
      url.searchParams.set('view', 'table');
      url.searchParams.set('table', name);
      Object.entries(params).forEach(([k, v]) => {
        if (v !== undefined) url.searchParams.set(k, String(v));
      });
      window.history.pushState(null, '', url);
    },
    [],
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
      const config = TABLE_CONFIGS[tableKey];
      if (!config) return;

      setTableLoading(true);
      setTimeRange(range);

      try {
        const fetcherArgs = [];
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

        openTable(
          title,
          tableKey === 'reorgs'
            ?
                'An L2 reorg occurs when the chain replaces previously published blocks. Depth shows how many blocks were replaced.'
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
          chart,
        );
      } catch (error) {
        console.error(`Failed to open ${tableKey} table:`, error);
        setTableLoading(false);
      }
    },
    [timeRange, setTimeRange, selectedSequencer, openTable, setTableUrl],
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
    openTpsTable,
    openSequencerDistributionTable,
  };
};
