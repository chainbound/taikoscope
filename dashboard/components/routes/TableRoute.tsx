import React, { useEffect, useState, useCallback } from 'react';
import { useParams, useSearchParams, useOutletContext, useNavigate } from 'react-router-dom';
import { TableView } from '../views/TableView';
import { DashboardHeader } from '../DashboardHeader';
import { TableViewState } from '../../hooks/useTableActions';
import { TimeRange } from '../../types';
import { TABLE_CONFIGS } from '../../config/tableConfig';
import { getSequencerAddress } from '../../sequencerConfig';
import { useDataFetcher } from '../../hooks/useDataFetcher';

interface DashboardContextType {
  timeRange: TimeRange;
  setTimeRange: (range: TimeRange) => void;
  selectedSequencer: string | null;
  setSelectedSequencer: (seq: string | null) => void;
  sequencerList: string[];
  chartsData: any;
  metricsData: any;
  refreshTimer: any;
}

export const TableRoute: React.FC = () => {
  const { tableType } = useParams<{ tableType: string }>();
  const [searchParams, setSearchParams] = useSearchParams();
  const navigate = useNavigate();

  const {
    timeRange,
    selectedSequencer,
    setSelectedSequencer,
    sequencerList,
    chartsData,
    metricsData,
    refreshTimer,
  } = useOutletContext<DashboardContextType>();

  // Override setTimeRange to update URL params for table route instead of navigating away
  const handleTimeRangeChange = useCallback((newRange: TimeRange) => {
    const newParams = new URLSearchParams(searchParams);
    if (newRange === '1h') {
      newParams.delete('range');
    } else {
      newParams.set('range', newRange);
    }
    setSearchParams(newParams, { replace: true });
  }, [searchParams, setSearchParams]);

  const [tableView, setTableView] = useState<TableViewState | undefined>(undefined);
  const [tableLoading, setTableLoading] = useState(false);

  // Get current time range from URL params, fallback to context
  const currentTimeRange = (searchParams.get('range') as TimeRange) || timeRange;

  const { handleManualRefresh } = useDataFetcher({
    timeRange: currentTimeRange,
    selectedSequencer,
    tableView: tableView || null,
    fetchMetricsData: metricsData.fetchMetricsData,
    updateChartsData: chartsData.updateChartsData,
    refreshRate: refreshTimer.refreshRate,
    updateLastRefresh: refreshTimer.updateLastRefresh,
  });

  useEffect(() => {
    const loadTable = async () => {
      if (!tableType) return;

      setTableLoading(true);
      setTableView(undefined);

      try {
        const range = currentTimeRange;

        if (tableType === 'tps') {
          // Handle TPS table - create from existing chart data
          const intervalMap = new Map<number, number>();
          chartsData.l2BlockTimeData.forEach((d: any) => {
            intervalMap.set(d.value, d.timestamp);
          });

          const data = chartsData.blockTxData
            .map((b: any) => {
              const ms = intervalMap.get(b.block);
              if (!ms) return null;
              return { block: b.block, tps: b.txs / (ms / 1000) };
            })
            .filter((d: any): d is { block: number; tps: number } => d !== null);

          setTableView({
            title: 'Transactions Per Second',
            description: undefined,
            columns: [
              { key: 'block', label: 'Block Number' },
              { key: 'tps', label: 'TPS' },
            ],
            rows: data.map((d: any) => ({ block: d.block, tps: d.tps.toFixed(2) })),
          });
        } else {
          // Handle other tables using config
          const config = TABLE_CONFIGS[tableType];
          if (!config) {
            throw new Error(`Unknown table type: ${tableType}`);
          }

          const fetcherArgs: any[] = [];
          const extraParams: Record<string, any> = {};

          if (tableType === 'sequencer-blocks') {
            const address = searchParams.get('address');
            if (address) {
              fetcherArgs.push(address);
              extraParams.address = address;
            }
          } else if (['l2-block-times', 'l2-gas-used'].includes(tableType)) {
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

          setTableView({
            title,
            description: tableType === 'reorgs'
              ? 'An L2 reorg occurs when the chain replaces previously published blocks. Depth shows how many blocks were replaced.'
              : undefined,
            columns: config.columns,
            rows: mappedData,
            chart,
          });
        }
      } catch (error) {
        console.error('Failed to load table:', error);
        metricsData.setErrorMessage(`Failed to load ${tableType} table. Please try again.`);
      } finally {
        setTableLoading(false);
      }
    };

    loadTable();
  }, [tableType, searchParams, currentTimeRange, selectedSequencer, chartsData, metricsData]);

  const handleBack = () => {
    navigate('/');
  };


  if (!tableView && !tableLoading) {
    return <div>Table not found</div>;
  }

  return (
    <>
      <DashboardHeader
        timeRange={currentTimeRange}
        onTimeRangeChange={handleTimeRangeChange}
        refreshRate={refreshTimer.refreshRate}
        onRefreshRateChange={refreshTimer.setRefreshRate}
        lastRefresh={refreshTimer.lastRefresh}
        onManualRefresh={handleManualRefresh}
        sequencers={sequencerList}
        selectedSequencer={selectedSequencer}
        onSequencerChange={setSelectedSequencer}
      />
      <TableView
        tableView={tableView}
        tableLoading={tableLoading}
        isNavigating={false}
        onBack={handleBack}
      />
    </>
  );
};